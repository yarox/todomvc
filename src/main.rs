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
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt,
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::SystemTime,
};
use tower_http::services::{ServeDir, ServeFile};
use uuid::Uuid;

type Db = Arc<RwLock<AppState>>;

#[derive(Debug, Default)]
struct AppState {
    todos: HashMap<Uuid, Todo>,
    num_all_items: u32,
    num_active_items: u32,
    num_completed_items: u32,
    selected_filter: TodoListFilter,
}

#[derive(Debug, Clone, PartialEq)]
struct Todo {
    id: Uuid,
    text: String,
    is_done: bool,
    created_at: SystemTime,
}

impl Default for Todo {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            text: String::new(),
            is_done: false,
            created_at: SystemTime::now(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct TodoCreate {
    text: String,
}

#[derive(Debug, Deserialize)]
struct TodoUpdate {
    text: Option<String>,
    is_done: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Default, Clone)]
pub enum TodoListFilter {
    #[default]
    All,
    Active,
    Completed,
}

impl fmt::Display for TodoListFilter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Completed => write!(f, "completed"),
            Self::Active => write!(f, "active"),
            Self::All => write!(f, "all"),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TodoListParams {
    filter: TodoListFilter,
}

#[derive(PartialEq, Props)]
struct TodoItemComponentProps {
    todo: Todo,
}

fn TodoItemComponent(cx: Scope<TodoItemComponentProps>) -> Element {
    cx.render(rsx! {
        div { class: "panel-block is-justify-content-space-between",
            input {
                id: "todo-done-{cx.props.todo.id}",
                "type": "checkbox",
                checked: if cx.props.todo.is_done { Some(true) } else { None },
                "hx-patch": "/todo/{cx.props.todo.id}",
                "hx-target": "closest .panel-block",
                "hx-swap": "outerHTML",
                "hx-vals": "js:{{is_done: document.getElementById('todo-done-{cx.props.todo.id}').checked}}"
            }
            p {
                class: "is-flex-grow-1",
                "hx-get": "/todo/{cx.props.todo.id}",
                "hx-target": "this",
                "hx-swap": "outerHTML",

                if cx.props.todo.is_done {
                    rsx!(s { cx.props.todo.text.clone() })
                } else {
                    rsx!(cx.props.todo.text.clone())
                }
            }
            button {
                class: "delete is-medium",
                "hx-delete": "/todo/{cx.props.todo.id}",
                "hx-target": "closest .panel-block",
                "hx-swap": "outerHTML"
            }
        }
    })
}

#[derive(PartialEq, Props)]
struct TodoEditComponentProps {
    item: Todo,
}

fn TodoEditComponent(cx: Scope<TodoEditComponentProps>) -> Element {
    cx.render(rsx! {
        form {
            class: "is-flex-grow-1",
            "hx-patch": "/todo/{cx.props.item.id}",
            "hx-target": "closest .panel-block",
            "hx-swap": "outerHTML",
            p { input { "type": "text", name: "text", value: "{cx.props.item.text}" } }
        }
    })
}

#[derive(PartialEq, Props)]
struct TodoListComponentProps {
    todos: Vec<Todo>,
}

fn TodoListComponent(cx: Scope<TodoListComponentProps>) -> Element {
    cx.render(rsx! {
        span { id: "todo-list",
            for todo in cx.props.todos.clone() {
                TodoItemComponent { todo: todo }
            }
        }
    })
}

#[derive(PartialEq, Props)]
struct TodoCounterComponentProps {
    filter: TodoListFilter,
    num_items: u32,
}

fn TodoCounterComponent(cx: Scope<TodoCounterComponentProps>) -> Element {
    cx.render(rsx! {
        span {
            id: "todo-counter-{cx.props.filter}",
            class: "tag is-rounded",
            "hx-swap-oob": true,
            "{cx.props.num_items}"
        }
    })
}

#[derive(PartialEq, Props)]
struct TodoDeleteCompletedComponentProps {
    is_disabled: bool,
}

fn TodoDeleteCompletedComponent(cx: Scope<TodoDeleteCompletedComponentProps>) -> Element {
    cx.render(rsx! {
        button {
            id: "todo-delete-completed",
            class: "button is-link is-outlined is-fullwidth",
            "hx-target": "#todo-list",
            "hx-swap": "outerHTML",
            "hx-delete": "/todo",
            "hx-swap-oob": true,
            disabled: if cx.props.is_disabled { Some(true) } else { None },
            "Delete completed"
        }
    })
}

#[tokio::main]
async fn main() {
    let shared_state = Arc::new(RwLock::new(AppState::default()));

    let app = Router::new()
        .nest_service("/", ServeFile::new("assets/index.html"))
        .nest_service("/assets", ServeDir::new("assets"))
        .route(
            "/todo",
            get(list_todo)
                .post(create_todo)
                .delete(delete_completed_todo),
        )
        .route(
            "/todo/:id",
            get(edit_todo).patch(update_todo).delete(delete_todo),
        )
        .with_state(shared_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn list_todo(
    State(db): State<Db>,
    Query(TodoListParams { filter }): Query<TodoListParams>,
) -> impl IntoResponse {
    db.write().unwrap().selected_filter = filter.clone();

    let state = db.read().unwrap();

    let mut todos = state
        .todos
        .values()
        .filter(|item| match filter {
            TodoListFilter::Completed => item.is_done,
            TodoListFilter::Active => !item.is_done,
            TodoListFilter::All => true,
        })
        .cloned()
        .collect::<Vec<_>>();

    todos.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Html(render_lazy(rsx! {
        TodoListComponent { todos: todos }

        TodoCounterComponent { filter: TodoListFilter::Completed, num_items: state.num_completed_items }
        TodoCounterComponent { filter: TodoListFilter::Active, num_items: state.num_active_items }
        TodoCounterComponent { filter: TodoListFilter::All, num_items: state.num_all_items }

        TodoDeleteCompletedComponent { is_disabled: state.num_completed_items == 0 }
    }))
}

async fn create_todo(State(db): State<Db>, Form(todo_new): Form<TodoCreate>) -> impl IntoResponse {
    let todo = Todo {
        text: todo_new.text,
        ..Default::default()
    };

    let mut state = db.write().unwrap();

    state.todos.insert(todo.id, todo.clone());
    state.num_active_items += 1;
    state.num_all_items += 1;

    drop(state);

    let state = db.read().unwrap();

    Html(render_lazy(rsx! {
        if state.selected_filter == TodoListFilter::Completed {
            rsx!("")
        } else {
            rsx!(TodoItemComponent { todo: todo })
        }

        TodoCounterComponent { filter: TodoListFilter::Active, num_items: state.num_active_items }
        TodoCounterComponent { filter: TodoListFilter::All, num_items: state.num_all_items }
    }))
}

async fn delete_completed_todo(State(db): State<Db>) -> impl IntoResponse {
    let mut state = db.write().unwrap();

    state.todos.retain(|_, v| !v.is_done);
    state.num_all_items -= state.num_completed_items;
    state.num_completed_items = 0;

    drop(state);

    let state = db.read().unwrap();
    let todos = if state.selected_filter == TodoListFilter::Completed {
        Vec::new()
    } else {
        let mut todos = state.todos.values().cloned().collect::<Vec<_>>();
        todos.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        todos
    };

    Html(render_lazy(rsx! {
        TodoListComponent { todos: todos }

        TodoCounterComponent { filter: TodoListFilter::Completed, num_items: state.num_completed_items }
        TodoCounterComponent { filter: TodoListFilter::All, num_items: state.num_all_items }

        TodoDeleteCompletedComponent { is_disabled: true }
    }))
}

async fn edit_todo(
    State(db): State<Db>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let item = db
        .read()
        .unwrap()
        .todos
        .get(&id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Html(render_lazy(rsx! { TodoEditComponent { item: item } })))
}

async fn update_todo(
    State(db): State<Db>,
    Path(id): Path<Uuid>,
    Form(todo_update): Form<TodoUpdate>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut todo = db
        .read()
        .unwrap()
        .todos
        .get(&id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut state = db.write().unwrap();

    if let Some(is_done) = todo_update.is_done {
        todo.is_done = is_done;

        if todo.is_done {
            state.num_completed_items += 1;
            state.num_active_items -= 1;
        } else {
            state.num_completed_items -= 1;
            state.num_active_items += 1;
        }
    }

    if let Some(text) = todo_update.text {
        todo.text = text;
    }

    state.todos.insert(todo.id, todo.clone());
    drop(state);

    let state = db.read().unwrap();

    Ok(Html(render_lazy(rsx! {
        match &state.selected_filter {
            TodoListFilter::Active if todo.is_done => rsx!(""),
            TodoListFilter::Active | TodoListFilter::All => rsx!(TodoItemComponent { todo: todo }),
            TodoListFilter::Completed if todo.is_done => rsx!(TodoItemComponent { todo: todo }),
            TodoListFilter::Completed => rsx!(""),
        }

        TodoCounterComponent { filter: TodoListFilter::Completed, num_items: state.num_completed_items }
        TodoCounterComponent { filter: TodoListFilter::Active, num_items: state.num_active_items }

        TodoDeleteCompletedComponent { is_disabled: state.num_completed_items == 0 }
    })))
}

async fn delete_todo(
    State(db): State<Db>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut state = db.write().unwrap();

    if let Some(item) = state.todos.remove(&id) {
        if item.is_done {
            state.num_completed_items -= 1;
        } else {
            state.num_active_items -= 1;
        }

        state.num_all_items -= 1;

        Ok(Html(render_lazy(rsx! {
            TodoCounterComponent { filter: TodoListFilter::Completed, num_items: state.num_completed_items }
            TodoCounterComponent { filter: TodoListFilter::Active, num_items: state.num_active_items }
            TodoCounterComponent { filter: TodoListFilter::All, num_items: state.num_all_items }

            TodoDeleteCompletedComponent { is_disabled: state.num_completed_items == 0 }
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
