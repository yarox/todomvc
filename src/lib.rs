#![allow(clippy::uninlined_format_args)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::unused_async)]
#![allow(non_snake_case)]

pub mod components;
pub mod models;
pub mod repository;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Form, Router,
};
use dioxus::prelude::*;
use dioxus_ssr::render_lazy;
use serde::Deserialize;
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
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
        .nest_service("/", ServeFile::new("assets/index.html"))
        .nest_service("/assets", ServeDir::new("assets"))
        .route(
            "/todo",
            get(list_todo)
                .post(create_todo)
                .patch(toggle_completed_todo)
                .delete(delete_completed_todo),
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

async fn list_todo(
    State(shared_state): State<SharedState>,
    Query(ListTodoParams { filter }): Query<ListTodoParams>,
) -> impl IntoResponse {
    shared_state.write().unwrap().selected_filter = filter;

    let state = shared_state.read().unwrap();
    let items = state.todo_repo.list(&filter);

    Html(render_lazy(rsx! {
        TodoListComponent { items: items }

        TodoTabsComponent {
            num_completed_items: state.todo_repo.num_completed_items,
            num_active_items: state.todo_repo.num_active_items,
            num_all_items: state.todo_repo.num_all_items
        }

        TodoDeleteCompletedComponent { is_disabled: state.todo_repo.num_completed_items == 0 }
        TodoToggleCompletedComponent { is_disabled: state.todo_repo.num_all_items == 0, action: state.toggle_action }
    }))
}

async fn create_todo(
    State(shared_state): State<SharedState>,
    Form(todo_create): Form<TodoCreate>,
) -> impl IntoResponse {
    let mut state = shared_state.write().unwrap();
    let item = state.todo_repo.create(&todo_create.text);

    state.toggle_action = TodoToggleAction::Check;

    Html(render_lazy(rsx! {
        if state.selected_filter != TodoListFilter::Completed {
            rsx!(TodoItemComponent { item: item })
        }

        TodoTabsComponent {
            num_completed_items: state.todo_repo.num_completed_items,
            num_active_items: state.todo_repo.num_active_items,
            num_all_items: state.todo_repo.num_all_items
        }

        TodoToggleCompletedComponent { is_disabled: false, action: state.toggle_action }
    }))
}

async fn toggle_completed_todo(
    State(shared_state): State<SharedState>,
    Query(ToggleCompletedTodoParams { action }): Query<ToggleCompletedTodoParams>,
) -> impl IntoResponse {
    let mut state = shared_state.write().unwrap();

    state.toggle_action = match action {
        TodoToggleAction::Uncheck => TodoToggleAction::Check,
        TodoToggleAction::Check => TodoToggleAction::Uncheck,
    };

    state.todo_repo.toggle_completed(&action);
    let items = state.todo_repo.list(&state.selected_filter);

    Html(render_lazy(rsx! {
        TodoListComponent { items: items }

        TodoTabsComponent {
            num_completed_items: state.todo_repo.num_completed_items,
            num_active_items: state.todo_repo.num_active_items,
            num_all_items: state.todo_repo.num_all_items
        }

        TodoDeleteCompletedComponent { is_disabled: state.todo_repo.num_completed_items == 0 }
        TodoToggleCompletedComponent { is_disabled: state.todo_repo.num_all_items == 0, action: state.toggle_action }
    }))
}

async fn delete_completed_todo(State(shared_state): State<SharedState>) -> impl IntoResponse {
    let mut state = shared_state.write().unwrap();

    state.toggle_action = TodoToggleAction::Check;
    state.todo_repo.delete_completed();

    let items = state.todo_repo.list(&state.selected_filter);

    Html(render_lazy(rsx! {
        TodoListComponent { items: items }

        TodoTabsComponent {
            num_completed_items: state.todo_repo.num_completed_items,
            num_active_items: state.todo_repo.num_active_items,
            num_all_items: state.todo_repo.num_all_items
        }

        TodoDeleteCompletedComponent { is_disabled: true }
        TodoToggleCompletedComponent { is_disabled: state.todo_repo.num_all_items == 0, action: state.toggle_action }
    }))
}

async fn edit_todo(
    State(shared_state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let item = shared_state.read().unwrap().todo_repo.get(&id)?;
    Ok(Html(render_lazy(rsx! { TodoEditComponent { item: item } })))
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
        }

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
