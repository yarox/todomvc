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

#[derive(Debug, Default)]
struct TodoRepo {
    num_completed_items: u32,
    num_active_items: u32,
    num_all_items: u32,
    todos: HashMap<Uuid, Todo>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq)]
enum TodoListFilter {
    Completed,
    Active,
    All,
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

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq)]
enum TodoToggleAction {
    Uncheck,
    Check,
}

impl fmt::Display for TodoToggleAction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Uncheck => write!(f, "Uncheck"),
            Self::Check => write!(f, "Check"),
        }
    }
}

#[derive(Debug)]
struct AppState {
    selected_filter: TodoListFilter,
    toggle_action: TodoToggleAction,
    todo_repo: TodoRepo,
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

type SharedState = Arc<RwLock<AppState>>;

#[derive(Debug, Clone, PartialEq)]
struct Todo {
    created_at: SystemTime,
    is_completed: bool,
    text: String,
    id: Uuid,
}

impl Todo {
    fn new(text: &str) -> Self {
        Self {
            created_at: SystemTime::now(),
            is_completed: false,
            text: String::from(text),
            id: Uuid::new_v4(),
        }
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
pub struct TodoListParams {
    filter: TodoListFilter,
}

#[derive(Debug, Deserialize)]
pub struct ToggleCompletedParams {
    action: TodoToggleAction,
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
                checked: if cx.props.todo.is_completed { Some(true) } else { None },
                "hx-patch": "/todo/{cx.props.todo.id}",
                "hx-target": "closest .panel-block",
                "hx-swap": "outerHTML",
                "hx-vals": "js:{{is_completed: document.getElementById('todo-done-{cx.props.todo.id}').checked}}"
            }
            p {
                class: "is-flex-grow-1",
                "hx-get": "/todo/{cx.props.todo.id}",
                "hx-trigger": "dblclick",
                "hx-target": "this",
                "hx-swap": "outerHTML",

                if cx.props.todo.is_completed {
                    rsx!(s { cx.props.todo.text.clone() })
                } else {
                    rsx!(cx.props.todo.text.clone())
                }
            }
            button {
                class: "delete is-medium ml-2",
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
            p {
                input {
                    class: "input",
                    "type": "text",
                    name: "text",
                    value: "{cx.props.item.text}",
                    autofocus: "true"
                }
            }
        }
    })
}

#[derive(PartialEq, Props)]
struct TodoListComponentProps {
    todos: Vec<Todo>,
}

fn TodoListComponent(cx: Scope<TodoListComponentProps>) -> Element {
    cx.render(rsx! {
        span {
            id: "todo-list",
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
            class: "button is-danger is-outlined is-fullwidth ml-1",
            "hx-target": "#todo-list",
            "hx-swap": "outerHTML",
            "hx-delete": "/todo",
            "hx-swap-oob": true,
            disabled: if cx.props.is_disabled { Some(true) } else { None },
            "Delete completed"
        }
    })
}

#[derive(PartialEq, Props)]
struct TodoToggleCompletedComponentProps {
    is_disabled: bool,
    action: TodoToggleAction,
}

fn TodoToggleCompletedComponent(cx: Scope<TodoToggleCompletedComponentProps>) -> Element {
    cx.render(rsx! {
        button {
            id: "todo-toggle-completed",
            class: "button is-link is-outlined is-fullwidth mr-1",
            "hx-target": "#todo-list",
            "hx-swap": "outerHTML",
            "hx-patch": "/todo?action={cx.props.action}",
            "hx-swap-oob": true,
            disabled: if cx.props.is_disabled { Some(true) } else { None },
            "{cx.props.action} all"
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
                .patch(toggle_completed_todo)
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
    State(shared_state): State<SharedState>,
    Query(TodoListParams { filter }): Query<TodoListParams>,
) -> impl IntoResponse {
    shared_state.write().unwrap().selected_filter = filter;

    let state = shared_state.read().unwrap();

    let mut todos = state
        .todo_repo
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

        TodoCounterComponent { filter: TodoListFilter::Completed, num_items: state.todo_repo.num_completed_items }
        TodoCounterComponent { filter: TodoListFilter::Active, num_items: state.todo_repo.num_active_items }
        TodoCounterComponent { filter: TodoListFilter::All, num_items: state.todo_repo.num_all_items }

        TodoDeleteCompletedComponent { is_disabled: state.todo_repo.num_completed_items == 0 }
        TodoToggleCompletedComponent { is_disabled: state.todo_repo.num_all_items == 0, action: state.toggle_action }
    }))
}

async fn create_todo(
    State(shared_state): State<SharedState>,
    Form(todo_new): Form<TodoCreate>,
) -> impl IntoResponse {
    let mut state = shared_state.write().unwrap();
    let todo = Todo::new(&todo_new.text);

    state.todo_repo.todos.insert(todo.id, todo.clone());
    state.toggle_action = TodoToggleAction::Check;
    state.todo_repo.num_active_items += 1;
    state.todo_repo.num_all_items += 1;

    Html(render_lazy(rsx! {
        if state.selected_filter == TodoListFilter::Completed {
            rsx!("")
        } else {
            rsx!(TodoItemComponent { todo: todo })
        }

        TodoCounterComponent { filter: TodoListFilter::Active, num_items: state.todo_repo.num_active_items }
        TodoCounterComponent { filter: TodoListFilter::All, num_items: state.todo_repo.num_all_items }
        TodoToggleCompletedComponent { is_disabled: false, action: state.toggle_action }
    }))
}

async fn toggle_completed_todo(
    State(shared_state): State<SharedState>,
    Query(ToggleCompletedParams { action }): Query<ToggleCompletedParams>,
) -> impl IntoResponse {
    if shared_state.read().unwrap().todo_repo.num_all_items == 0 {
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

    for todo in state.todo_repo.todos.values_mut() {
        todo.is_completed = is_completed;
    }

    if is_completed {
        state.todo_repo.num_completed_items = state.todo_repo.num_all_items;
        state.todo_repo.num_active_items = 0;
    } else {
        state.todo_repo.num_completed_items = 0;
        state.todo_repo.num_active_items = state.todo_repo.num_all_items;
    }

    drop(state);

    let state = shared_state.read().unwrap();
    let selected_filter = &state.selected_filter;

    let mut todos = state
        .todo_repo
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

        TodoCounterComponent { filter: TodoListFilter::Completed, num_items: state.todo_repo.num_completed_items }
        TodoCounterComponent { filter: TodoListFilter::Active, num_items: state.todo_repo.num_active_items }
        TodoCounterComponent { filter: TodoListFilter::All, num_items: state.todo_repo.num_all_items }

        TodoDeleteCompletedComponent { is_disabled: state.todo_repo.num_completed_items == 0 }
        TodoToggleCompletedComponent { is_disabled: state.todo_repo.num_all_items == 0, action: state.toggle_action }
    }))
}

async fn delete_completed_todo(State(shared_state): State<SharedState>) -> impl IntoResponse {
    let mut state = shared_state.write().unwrap();

    state.todo_repo.todos.retain(|_, v| !v.is_completed);
    state.todo_repo.num_all_items -= state.todo_repo.num_completed_items;
    state.toggle_action = TodoToggleAction::Check;
    state.todo_repo.num_completed_items = 0;

    let todos = if state.selected_filter == TodoListFilter::Completed {
        Vec::new()
    } else {
        let mut todos = state.todo_repo.todos.values().cloned().collect::<Vec<_>>();
        todos.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        todos
    };

    Html(render_lazy(rsx! {
        TodoListComponent { todos: todos }

        TodoCounterComponent { filter: TodoListFilter::Completed, num_items: state.todo_repo.num_completed_items }
        TodoCounterComponent { filter: TodoListFilter::All, num_items: state.todo_repo.num_all_items }

        TodoDeleteCompletedComponent { is_disabled: true }
        TodoToggleCompletedComponent { is_disabled: state.todo_repo.num_all_items == 0, action: state.toggle_action }
    }))
}

async fn edit_todo(
    State(shared_state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let item = shared_state
        .read()
        .unwrap()
        .todo_repo
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
        .todo_repo
        .todos
        .get(&id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut state = shared_state.write().unwrap();

    if let Some(is_completed) = todo_update.is_completed {
        todo.is_completed = is_completed;

        if todo.is_completed {
            state.todo_repo.num_completed_items += 1;
            state.todo_repo.num_active_items -= 1;
        } else {
            state.todo_repo.num_completed_items -= 1;
            state.todo_repo.num_active_items += 1;
        }
    }

    if let Some(text) = todo_update.text {
        todo.text = text;
    }

    state.todo_repo.todos.insert(todo.id, todo.clone());

    Ok(Html(render_lazy(rsx! {
        match &state.selected_filter {
            TodoListFilter::Active if todo.is_completed => rsx!(""),
            TodoListFilter::Active | TodoListFilter::All => rsx!(TodoItemComponent { todo: todo }),
            TodoListFilter::Completed if todo.is_completed => rsx!(TodoItemComponent { todo: todo }),
            TodoListFilter::Completed => rsx!(""),
        }

        TodoCounterComponent { filter: TodoListFilter::Completed, num_items: state.todo_repo.num_completed_items }
        TodoCounterComponent { filter: TodoListFilter::Active, num_items: state.todo_repo.num_active_items }

        TodoDeleteCompletedComponent { is_disabled: state.todo_repo.num_completed_items == 0 }
    })))
}

async fn delete_todo(
    State(shared_state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut state = shared_state.write().unwrap();

    if let Some(item) = state.todo_repo.todos.remove(&id) {
        if item.is_completed {
            state.todo_repo.num_completed_items -= 1;
        } else {
            state.todo_repo.num_active_items -= 1;
        }

        state.todo_repo.num_all_items -= 1;

        if state.todo_repo.num_all_items == 0 {
            state.toggle_action = TodoToggleAction::Check;
        } else {
            state.toggle_action = TodoToggleAction::Uncheck;
        }

        Ok(Html(render_lazy(rsx! {
            TodoCounterComponent { filter: TodoListFilter::Completed, num_items: state.todo_repo.num_completed_items }
            TodoCounterComponent { filter: TodoListFilter::Active, num_items: state.todo_repo.num_active_items }
            TodoCounterComponent { filter: TodoListFilter::All, num_items: state.todo_repo.num_all_items }

            TodoDeleteCompletedComponent { is_disabled: state.todo_repo.num_completed_items == 0 }
            TodoToggleCompletedComponent { is_disabled: state.todo_repo.num_all_items == 0, action: state.toggle_action }
        })))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
