#![allow(clippy::uninlined_format_args)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::unused_async)]
#![allow(non_snake_case)]

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Form, Router,
};
use dioxus::prelude::*;
use dioxus_ssr::render_lazy;
use serde::Deserialize;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

pub mod components;
pub mod models;

use components::*;
use models::*;

#[derive(Debug, Default)]
struct TodoDb {
    num_completed_items: u32,
    num_active_items: u32,
    num_all_items: u32,
    todos: HashMap<Uuid, Todo>,
}

#[derive(Debug)]
struct AppState {
    selected_filter: TodoListFilter,
    toggle_action: TodoToggleAction,
    todo_db: TodoDb,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            selected_filter: TodoListFilter::All,
            toggle_action: TodoToggleAction::Check,
            todo_db: TodoDb::default(),
        }
    }
}

type SharedState = Arc<RwLock<AppState>>;

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
pub struct TodoListParams {
    filter: TodoListFilter,
}

#[derive(Debug, Deserialize)]
pub struct ToggleCompletedParams {
    action: TodoToggleAction,
}

#[tokio::main]
async fn main() {
    let shared_state = Arc::new(RwLock::new(AppState::default()));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "todomvc=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = Router::new()
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
        .with_state(shared_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn list_todo(
    State(shared_state): State<SharedState>,
    Query(TodoListParams { filter }): Query<TodoListParams>,
) -> impl IntoResponse {
    shared_state.write().unwrap().selected_filter = filter;

    let state = shared_state.read().unwrap();

    let mut todos = state
        .todo_db
        .todos
        .values()
        .filter(|item| match filter {
            TodoListFilter::Completed => item.is_completed,
            TodoListFilter::Active => !item.is_completed,
            TodoListFilter::All => true,
        })
        .cloned()
        .collect::<Vec<_>>();

    todos.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Html(render_lazy(rsx! {
        TodoListComponent { todos: todos }

        TodoTabsComponent {
            num_completed_items: state.todo_db.num_completed_items,
            num_active_items: state.todo_db.num_active_items,
            num_all_items: state.todo_db.num_all_items
        }

        TodoDeleteCompletedComponent { is_disabled: state.todo_db.num_completed_items == 0 }
        TodoToggleCompletedComponent { is_disabled: state.todo_db.num_all_items == 0, action: state.toggle_action }
    }))
}

async fn create_todo(
    State(shared_state): State<SharedState>,
    Form(todo_new): Form<TodoCreate>,
) -> impl IntoResponse {
    let mut state = shared_state.write().unwrap();
    let todo = Todo::new(&todo_new.text);

    state.todo_db.todos.insert(todo.id, todo.clone());
    state.toggle_action = TodoToggleAction::Check;
    state.todo_db.num_active_items += 1;
    state.todo_db.num_all_items += 1;

    Html(render_lazy(rsx! {
        if state.selected_filter == TodoListFilter::Completed {
            rsx!("")
        } else {
            rsx!(TodoItemComponent { todo: todo })
        }

        TodoTabsComponent {
            num_completed_items: state.todo_db.num_completed_items,
            num_active_items: state.todo_db.num_active_items,
            num_all_items: state.todo_db.num_all_items
        }

        TodoToggleCompletedComponent { is_disabled: false, action: state.toggle_action }
    }))
}

async fn toggle_completed_todo(
    State(shared_state): State<SharedState>,
    Query(ToggleCompletedParams { action }): Query<ToggleCompletedParams>,
) -> impl IntoResponse {
    if shared_state.read().unwrap().todo_db.num_all_items == 0 {
        return Html(render_lazy(rsx! { TodoListComponent { todos: Vec::new() }}));
    }

    let mut state = shared_state.write().unwrap();
    let is_completed;

    match action {
        TodoToggleAction::Uncheck => {
            state.toggle_action = TodoToggleAction::Check;
            is_completed = false;
        }
        TodoToggleAction::Check => {
            state.toggle_action = TodoToggleAction::Uncheck;
            is_completed = true;
        }
    };

    for todo in state.todo_db.todos.values_mut() {
        todo.is_completed = is_completed;
    }

    if is_completed {
        state.todo_db.num_completed_items = state.todo_db.num_all_items;
        state.todo_db.num_active_items = 0;
    } else {
        state.todo_db.num_completed_items = 0;
        state.todo_db.num_active_items = state.todo_db.num_all_items;
    }

    let selected_filter = &state.selected_filter;

    let mut todos = state
        .todo_db
        .todos
        .values()
        .filter(|item| match selected_filter {
            TodoListFilter::Completed => item.is_completed,
            TodoListFilter::Active => !item.is_completed,
            TodoListFilter::All => true,
        })
        .cloned()
        .collect::<Vec<_>>();

    todos.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Html(render_lazy(rsx! {
        TodoListComponent { todos: todos }

        TodoTabsComponent {
            num_completed_items: state.todo_db.num_completed_items,
            num_active_items: state.todo_db.num_active_items,
            num_all_items: state.todo_db.num_all_items
        }

        TodoDeleteCompletedComponent { is_disabled: state.todo_db.num_completed_items == 0 }
        TodoToggleCompletedComponent { is_disabled: state.todo_db.num_all_items == 0, action: state.toggle_action }
    }))
}

async fn delete_completed_todo(State(shared_state): State<SharedState>) -> impl IntoResponse {
    let mut state = shared_state.write().unwrap();

    state.todo_db.todos.retain(|_, v| !v.is_completed);
    state.todo_db.num_all_items -= state.todo_db.num_completed_items;
    state.toggle_action = TodoToggleAction::Check;
    state.todo_db.num_completed_items = 0;

    let todos = if state.selected_filter == TodoListFilter::Completed {
        Vec::new()
    } else {
        let mut todos = state.todo_db.todos.values().cloned().collect::<Vec<_>>();
        todos.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        todos
    };

    Html(render_lazy(rsx! {
        TodoListComponent { todos: todos }

        TodoTabsComponent {
            num_completed_items: state.todo_db.num_completed_items,
            num_active_items: state.todo_db.num_active_items,
            num_all_items: state.todo_db.num_all_items
        }

        TodoDeleteCompletedComponent { is_disabled: true }
        TodoToggleCompletedComponent { is_disabled: state.todo_db.num_all_items == 0, action: state.toggle_action }
    }))
}

async fn edit_todo(
    State(shared_state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let item = shared_state
        .read()
        .unwrap()
        .todo_db
        .todos
        .get(&id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Html(render_lazy(rsx! { TodoEditComponent { item: item } })))
}

async fn update_todo(
    State(shared_state): State<SharedState>,
    Path(id): Path<Uuid>,
    Form(todo_update): Form<TodoUpdate>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut todo = shared_state
        .read()
        .unwrap()
        .todo_db
        .todos
        .get(&id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut state = shared_state.write().unwrap();

    if let Some(is_completed) = todo_update.is_completed {
        todo.is_completed = is_completed;

        if todo.is_completed {
            state.todo_db.num_completed_items += 1;
            state.todo_db.num_active_items -= 1;
        } else {
            state.todo_db.num_completed_items -= 1;
            state.todo_db.num_active_items += 1;
        }
    }

    if let Some(text) = todo_update.text {
        todo.text = text;
    }

    state.todo_db.todos.insert(todo.id, todo.clone());

    Ok(Html(render_lazy(rsx! {
        match &state.selected_filter {
            TodoListFilter::Active if todo.is_completed => rsx!(""),
            TodoListFilter::Active | TodoListFilter::All => rsx!(TodoItemComponent { todo: todo }),
            TodoListFilter::Completed if todo.is_completed => rsx!(TodoItemComponent { todo: todo }),
            TodoListFilter::Completed => rsx!(""),
        }

        TodoTabsComponent {
            num_completed_items: state.todo_db.num_completed_items,
            num_active_items: state.todo_db.num_active_items,
            num_all_items: state.todo_db.num_all_items
        }

        TodoDeleteCompletedComponent { is_disabled: state.todo_db.num_completed_items == 0 }
    })))
}

async fn delete_todo(
    State(shared_state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut state = shared_state.write().unwrap();

    if let Some(item) = state.todo_db.todos.remove(&id) {
        if item.is_completed {
            state.todo_db.num_completed_items -= 1;
        } else {
            state.todo_db.num_active_items -= 1;
        }

        state.todo_db.num_all_items -= 1;

        if state.todo_db.num_all_items == 0 {
            state.toggle_action = TodoToggleAction::Check;
        } else {
            state.toggle_action = TodoToggleAction::Uncheck;
        }

        Ok(Html(render_lazy(rsx! {
            TodoTabsComponent {
                num_completed_items: state.todo_db.num_completed_items,
                num_active_items: state.todo_db.num_active_items,
                num_all_items: state.todo_db.num_all_items
            }

            TodoDeleteCompletedComponent { is_disabled: state.todo_db.num_completed_items == 0 }
            TodoToggleCompletedComponent { is_disabled: state.todo_db.num_all_items == 0, action: state.toggle_action }
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
