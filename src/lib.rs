#![allow(clippy::uninlined_format_args)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::unused_async)]
#![allow(non_snake_case)]

pub mod components;
pub mod models;
pub mod repository;

use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Form, Router,
};
use dioxus::prelude::*;
use dioxus_ssr::render_lazy;
use models::Todo;
use serde::Deserialize;
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use crate::components::{
    TodoDeleteCompletedComponent, TodoEditComponent, TodoItemComponent, TodoListComponent,
    TodoTabsComponent, TodoToggleCompletedComponent,
};
use crate::models::{TodoListFilter, TodoToggleAction};
use crate::repository::{TodoRepo, TodoRepoError};

#[derive(Debug)]
pub struct AppState {
    pub selected_filter: TodoListFilter,
    pub toggle_action: TodoToggleAction,
    pub todo_repo: TodoRepo,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            selected_filter: TodoListFilter::All,
            toggle_action: TodoToggleAction::Check,
            todo_repo: TodoRepo::default(),
        }
    }
}

pub type SharedState = Arc<RwLock<AppState>>;

enum AppError {
    TodoRepo(TodoRepoError),
}

impl From<TodoRepoError> for AppError {
    fn from(inner: TodoRepoError) -> Self {
        Self::TodoRepo(inner)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::TodoRepo(TodoRepoError::NotFound) => (StatusCode::NOT_FOUND, "Todo not found"),
        };

        (status, message).into_response()
    }
}

#[derive(Debug, Deserialize)]
struct TodoCreate {
    text: String,
}

#[derive(Debug, Deserialize)]
struct TodoUpdate {
    is_completed: Option<bool>,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ListTodoParams {
    filter: TodoListFilter,
}

#[derive(Debug, Deserialize)]
pub struct ToggleCompletedTodoParams {
    action: TodoToggleAction,
}

pub fn app(shared_state: SharedState) -> Router {
    Router::new()
        .nest_service("/assets", ServeDir::new("assets"))
        .route("/", get(get_index))
        .route(
            "/todo",
            get(list_todos)
                .post(create_todo)
                .patch(toggle_completed_todos)
                .delete(delete_completed_todos),
        )
        .route(
            "/todo/:id",
            get(edit_todo).patch(update_todo).delete(delete_todo),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(shared_state)
}

pub async fn run() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "todomvc=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::debug!("listening on {}", addr);

    let shared_state = SharedState::default();
    let app = app(shared_state);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Template)]
#[template(path = "responses/index.html")]
struct GetIndexResponse;

async fn get_index() -> Result<GetIndexResponse, AppError> {
    Ok(GetIndexResponse)
}

#[derive(Template)]
#[template(path = "responses/list_todos.html")]
struct ListTodosResponse {
    num_completed_items: u32,
    num_active_items: u32,
    num_all_items: u32,
    is_disabled_delete: bool,
    is_disabled_toggle: bool,
    action: TodoToggleAction,
    items: Vec<Todo>,
}

#[derive(Debug, Deserialize)]
struct ListTodosQuery {
    filter: TodoListFilter,
}

async fn list_todos(
    State(shared_state): State<SharedState>,
    Query(ListTodosQuery { filter }): Query<ListTodosQuery>,
) -> Result<ListTodosResponse, AppError> {
    shared_state.write().unwrap().selected_filter = filter;

    let state = shared_state.read().unwrap();
    let items = state.todo_repo.list(&filter);

    Ok(ListTodosResponse {
        num_completed_items: state.todo_repo.num_completed_items,
        num_active_items: state.todo_repo.num_active_items,
        num_all_items: state.todo_repo.num_all_items,
        is_disabled_delete: state.todo_repo.num_completed_items == 0,
        is_disabled_toggle: state.todo_repo.num_all_items == 0,
        action: state.toggle_action,
        items,
    })
}

#[derive(Template)]
#[template(path = "responses/create_todo.html")]
struct CreateTodoResponse {
    num_completed_items: u32,
    num_active_items: u32,
    num_all_items: u32,
    is_completed_selected: bool,
    is_disabled_toggle: bool,
    action: TodoToggleAction,
    item: Todo,
}

#[derive(Debug, Deserialize)]
struct CreateTodoForm {
    text: String,
}

async fn create_todo(
    State(shared_state): State<SharedState>,
    Form(CreateTodoForm { text }): Form<CreateTodoForm>,
) -> Result<CreateTodoResponse, AppError> {
    let mut state = shared_state.write().unwrap();
    let item = state.todo_repo.create(&text);

    state.toggle_action = TodoToggleAction::Check;

    Ok(CreateTodoResponse {
        num_completed_items: state.todo_repo.num_completed_items,
        num_active_items: state.todo_repo.num_active_items,
        num_all_items: state.todo_repo.num_all_items,
        is_completed_selected: state.selected_filter == TodoListFilter::Completed,
        is_disabled_toggle: false,
        action: state.toggle_action,
        item,
    })
}

#[derive(Template)]
#[template(path = "responses/toggle_completed_todos.html")]
struct ToggleCompletedTodosResponse {
    num_completed_items: u32,
    num_active_items: u32,
    num_all_items: u32,
    is_disabled_delete: bool,
    is_disabled_toggle: bool,
    action: TodoToggleAction,
    items: Vec<Todo>,
}

#[derive(Debug, Deserialize)]
struct ToggleCompletedTodosQuery {
    action: TodoToggleAction,
}

async fn toggle_completed_todos(
    State(shared_state): State<SharedState>,
    Query(ToggleCompletedTodosQuery { action }): Query<ToggleCompletedTodosQuery>,
) -> Result<ToggleCompletedTodosResponse, AppError> {
    let mut state = shared_state.write().unwrap();

    state.toggle_action = match action {
        TodoToggleAction::Uncheck => TodoToggleAction::Check,
        TodoToggleAction::Check => TodoToggleAction::Uncheck,
    };

    state.todo_repo.toggle_completed(&action);
    let items = state.todo_repo.list(&state.selected_filter);

    Ok(ToggleCompletedTodosResponse {
        num_completed_items: state.todo_repo.num_completed_items,
        num_active_items: state.todo_repo.num_active_items,
        num_all_items: state.todo_repo.num_all_items,
        is_disabled_delete: state.todo_repo.num_completed_items == 0,
        is_disabled_toggle: state.todo_repo.num_all_items == 0,
        action: state.toggle_action,
        items,
    })
}

#[derive(Template)]
#[template(path = "responses/delete_completed_todos.html")]
struct DeleteCompletedTodosResponse {
    num_completed_items: u32,
    num_active_items: u32,
    num_all_items: u32,
    is_disabled_delete: bool,
    is_disabled_toggle: bool,
    action: TodoToggleAction,
    items: Vec<Todo>,
}

async fn delete_completed_todos(
    State(shared_state): State<SharedState>,
) -> Result<DeleteCompletedTodosResponse, AppError> {
    let mut state = shared_state.write().unwrap();

    state.toggle_action = TodoToggleAction::Check;
    state.todo_repo.delete_completed();

    let items = state.todo_repo.list(&state.selected_filter);

    Ok(DeleteCompletedTodosResponse {
        num_completed_items: state.todo_repo.num_completed_items,
        num_active_items: state.todo_repo.num_active_items,
        num_all_items: state.todo_repo.num_all_items,
        is_disabled_delete: true,
        is_disabled_toggle: state.todo_repo.num_all_items == 0,
        action: state.toggle_action,
        items,
    })
}

#[derive(Template)]
#[template(path = "responses/edit_todo.html")]
struct EditTodoResponse {
    item: Todo,
}

async fn edit_todo(
    State(shared_state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<EditTodoResponse, AppError> {
    let item = shared_state.read().unwrap().todo_repo.get(&id)?;
    Ok(EditTodoResponse { item })
}

async fn update_todo(
    State(shared_state): State<SharedState>,
    Path(id): Path<Uuid>,
    Form(todo_update): Form<TodoUpdate>,
) -> Result<impl IntoResponse, AppError> {
    let mut state = shared_state.write().unwrap();
    let item = state
        .todo_repo
        .update(&id, todo_update.text, todo_update.is_completed)?;

    state.toggle_action = if state.todo_repo.num_completed_items == state.todo_repo.num_all_items {
        TodoToggleAction::Uncheck
    } else {
        TodoToggleAction::Check
    };

    Ok(Html(render_lazy(rsx! {
        match state.selected_filter {
            TodoListFilter::Active if item.is_completed => rsx!(""),
            TodoListFilter::Active | TodoListFilter::All => rsx!(TodoItemComponent { item: item }),
            TodoListFilter::Completed if item.is_completed => rsx!(TodoItemComponent { item: item }),
            TodoListFilter::Completed => rsx!(""),
        },

        TodoTabsComponent {
            num_completed_items: state.todo_repo.num_completed_items,
            num_active_items: state.todo_repo.num_active_items,
            num_all_items: state.todo_repo.num_all_items
        }

        TodoDeleteCompletedComponent { is_disabled: state.todo_repo.num_completed_items == 0 }
        TodoToggleCompletedComponent { is_disabled: state.todo_repo.num_all_items == 0, action: state.toggle_action }
    })))
}

async fn delete_todo(
    State(shared_state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let mut state = shared_state.write().unwrap();
    state.todo_repo.delete(&id)?;

    state.toggle_action = if state.todo_repo.num_all_items == 0 {
        TodoToggleAction::Check
    } else {
        TodoToggleAction::Uncheck
    };

    Ok(Html(render_lazy(rsx! {
        TodoTabsComponent {
            num_completed_items: state.todo_repo.num_completed_items,
            num_active_items: state.todo_repo.num_active_items,
            num_all_items: state.todo_repo.num_all_items
        }

        TodoDeleteCompletedComponent { is_disabled: state.todo_repo.num_completed_items == 0 }
        TodoToggleCompletedComponent { is_disabled: state.todo_repo.num_all_items == 0, action: state.toggle_action }
    })))
}
