#![allow(clippy::uninlined_format_args)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::unused_async)]
#![allow(non_snake_case)]

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Html,
    routing::get,
    Form, Router,
};
use dioxus::prelude::*;
use dioxus_ssr::render_lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::SystemTime,
};
use tower_http::services::{ServeDir, ServeFile};
use uuid::Uuid;

#[derive(Debug, Default)]
struct AppState {
    todos: HashMap<Uuid, Todo>,
    num_all_items: u32,
    num_active_items: u32,
    num_completed_items: u32,
    selected_filter: TodoListFilter,
}

type AppError = (StatusCode, String);

#[derive(Debug, Clone, PartialEq)]
struct Todo {
    id: Uuid,
    text: String,
    is_done: bool,
    created_at: SystemTime,
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub enum TodoListFilter {
    #[default]
    All,
    Active,
    Completed,
}

#[derive(Debug, Deserialize)]
pub struct TodoListParams {
    filter: TodoListFilter,
}

#[derive(PartialEq, Props)]
struct TodoItemComponentProps {
    item: Todo,
}

fn TodoItemComponent(cx: Scope<TodoItemComponentProps>) -> Element {
    cx.render(rsx! {
        div {
            class: "panel-block is-justify-content-space-between",
            input {
                id: "todo-done-{cx.props.item.id}",
                "type": "checkbox",
                checked: if cx.props.item.is_done {Some(true)} else {None},
                "hx-patch": "/todo/{cx.props.item.id}",
                "hx-target": "closest .panel-block",
                "hx-swap": "outerHTML",
                "hx-vals": "js:{{is_done: document.getElementById('todo-done-{cx.props.item.id}').checked}}"
            },
            p {
                class: "is-flex-grow-1",
                "hx-get": "/todo/{cx.props.item.id}",
                "hx-target": "this",
                "hx-swap": "outerHTML",

                if cx.props.item.is_done {
                    rsx!(s { cx.props.item.text.clone() })
                } else {
                    rsx!(cx.props.item.text.clone())
                }
            },
            button {
                class: "delete is-medium",
                "hx-delete": "/todo/{cx.props.item.id}",
                "hx-target": "closest .panel-block",
                "hx-swap": "outerHTML",
            },
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
          p {
            input {
                "type": "text",
                name: "text",
                value: "{cx.props.item.text}",
            },
          },
      }
    })
}

#[derive(PartialEq, Props)]
struct TodoListComponentProps {
    items: Vec<Todo>,
}

fn TodoListComponent(cx: Scope<TodoListComponentProps>) -> Element {
    cx.render(rsx! {
        span {
            id: "todo-list",
            for item in cx.props.items.clone() {
                TodoItemComponent {
                    item: item
                }
            }
      }
    })
}

#[derive(PartialEq, Props)]
struct TodoCounterComponentProps {
    name: String,
    num_items: u32,
}

fn TodoCounterComponent(cx: Scope<TodoCounterComponentProps>) -> Element {
    cx.render(rsx! {
        span {
            id: "todo-counter-{cx.props.name}",
            class: "tag is-rounded",
            "hx-swap-oob": true,
            "{cx.props.num_items}",
        },
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
            disabled: if cx.props.is_disabled {Some(true)} else {None},
            "Delete completed"
        },
    })
}

#[tokio::main]
async fn main() {
    let shared_state = Arc::new(RwLock::new(AppState {
        todos: HashMap::default(),
        num_all_items: 0,
        num_active_items: 0,
        num_completed_items: 0,
        selected_filter: TodoListFilter::All,
    }));

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

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn list_todo(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(TodoListParams { filter }): Query<TodoListParams>,
) -> Result<Html<String>, AppError> {
    let mut items = state
        .read()
        .unwrap()
        .todos
        .values()
        .filter(|item| match filter {
            TodoListFilter::Completed => item.is_done,
            TodoListFilter::Active => !item.is_done,
            TodoListFilter::All => true,
        })
        .cloned()
        .collect::<Vec<_>>();

    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    state.write().unwrap().selected_filter = filter;

    let lock = state.read().unwrap();

    Ok(Html(render_lazy(rsx! {
        TodoListComponent {
            items: items
        },
        TodoCounterComponent {
            name: "all".to_string(),
            num_items: lock.num_all_items,
        },
        TodoCounterComponent {
            name: "active".to_string(),
            num_items: lock.num_active_items,
        },
        TodoCounterComponent {
            name: "completed".to_string(),
            num_items: lock.num_completed_items,
        },
        TodoDeleteCompletedComponent {
            is_disabled: lock.num_completed_items == 0,
        },
    })))
}

async fn create_todo(
    State(state): State<Arc<RwLock<AppState>>>,
    Form(todo_new): Form<TodoCreate>,
) -> Result<Html<String>, AppError> {
    let item = Todo {
        id: Uuid::new_v4(),
        text: todo_new.text,
        is_done: false,
        created_at: SystemTime::now(),
    };

    {
        let mut lock = state.write().unwrap();

        lock.todos.insert(item.id, item.clone());

        lock.num_all_items += 1;
        lock.num_active_items += 1;
    }

    let lock = state.read().unwrap();
    let todo_item_component = if lock.selected_filter == TodoListFilter::Completed {
        rsx!("")
    } else {
        rsx!(TodoItemComponent { item: item })
    };

    Ok(Html(render_lazy(rsx! {
        todo_item_component,
        TodoCounterComponent {
            name: "all".to_string(),
            num_items: lock.num_all_items,
        },
        TodoCounterComponent {
            name: "active".to_string(),
            num_items: lock.num_active_items,
        },
    })))
}

async fn delete_completed_todo(
    State(state): State<Arc<RwLock<AppState>>>,
) -> Result<Html<String>, AppError> {
    state.write().unwrap().todos.retain(|_, v| !v.is_done);

    let items = if state.read().unwrap().selected_filter == TodoListFilter::Completed {
        Vec::new()
    } else {
        let mut items = state
            .read()
            .unwrap()
            .todos
            .values()
            .cloned()
            .collect::<Vec<_>>();

        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items
    };

    let num_completed_items = state.read().unwrap().num_completed_items;

    state.write().unwrap().num_all_items -= num_completed_items;
    state.write().unwrap().num_completed_items = 0;

    Ok(Html(render_lazy(rsx! {
        TodoListComponent {
            items: items
        },
        TodoCounterComponent {
            name: "completed".to_string(),
            num_items: 0,
        },TodoCounterComponent {
            name: "all".to_string(),
            num_items: state.read().unwrap().num_all_items
        },
        TodoDeleteCompletedComponent {
            is_disabled: true,
        },
    })))
}

async fn edit_todo(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, AppError> {
    let lock = state.read().unwrap();

    if let Some(item) = lock.todos.get(&id) {
        Ok(Html(render_lazy(rsx! {
            TodoEditComponent {
                item: item.clone()
            }
        })))
    } else {
        Err((StatusCode::NOT_FOUND, format!("Todo not found: {}", id)))
    }
}

async fn update_todo(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(id): Path<Uuid>,
    Form(todo_update): Form<TodoUpdate>,
) -> Result<Html<String>, AppError> {
    let mut lock = state.write().unwrap();

    if let Some(item) = lock.todos.get_mut(&id) {
        item.is_done = todo_update.is_done.unwrap_or(item.is_done);
        item.text = todo_update.text.unwrap_or_else(|| item.text.clone());

        let item = item.clone();

        if todo_update.is_done.is_some() {
            if item.is_done {
                lock.num_completed_items += 1;
                lock.num_active_items -= 1;
            } else {
                lock.num_completed_items -= 1;
                lock.num_active_items += 1;
            }
        }

        let todo_item_component = match &lock.selected_filter {
            TodoListFilter::Active if item.is_done => rsx!(""),
            TodoListFilter::Active | TodoListFilter::All => rsx!(TodoItemComponent { item: item }),
            TodoListFilter::Completed if item.is_done => rsx!(TodoItemComponent { item: item }),
            TodoListFilter::Completed => rsx!(""),
        };

        Ok(Html(render_lazy(rsx! {
            todo_item_component,
            TodoCounterComponent {
                name: "active".to_string(),
                num_items: lock.num_active_items,
            },
            TodoCounterComponent {
                name: "completed".to_string(),
                num_items: lock.num_completed_items,
            },
            TodoDeleteCompletedComponent {
                is_disabled: lock.num_completed_items == 0,
            },
        })))
    } else {
        Err((StatusCode::NOT_FOUND, format!("Todo not Found: {}", id)))
    }
}

async fn delete_todo(
    State(state): State<Arc<RwLock<AppState>>>,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, AppError> {
    let mut lock = state.write().unwrap();

    if let Some(item) = lock.todos.remove(&id) {
        if item.is_done {
            lock.num_completed_items -= 1;
        } else {
            lock.num_active_items -= 1;
        }

        lock.num_all_items -= 1;

        Ok(Html(render_lazy(rsx! {
            TodoCounterComponent {
                name: "all".to_string(),
                num_items: lock.num_all_items,
            },
            TodoCounterComponent {
                name: "active".to_string(),
                num_items: lock.num_active_items,
            },
            TodoCounterComponent {
                name: "completed".to_string(),
                num_items: lock.num_completed_items,
            },
            TodoDeleteCompletedComponent {
                is_disabled: lock.num_completed_items == 0,
            },
        })))
    } else {
        Err((StatusCode::NOT_FOUND, format!("Todo not found: {}", id)))
    }
}
