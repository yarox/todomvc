use crate::models::{Todo, TodoListFilter, TodoToggleAction};
use dioxus::prelude::*;

#[derive(Props, PartialEq, Eq)]
pub struct TodoItemComponentProps {
    todo: Todo,
}

pub fn TodoItemComponent(cx: Scope<TodoItemComponentProps>) -> Element {
    cx.render(rsx! {
        div { class: "panel-block is-justify-content-space-between todo-item",
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

#[derive(Props, PartialEq, Eq)]
pub struct TodoEditComponentProps {
    item: Todo,
}

pub fn TodoEditComponent(cx: Scope<TodoEditComponentProps>) -> Element {
    cx.render(rsx! {
        form {
            class: "is-flex-grow-1 todo-edit",
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

#[derive(Props, PartialEq, Eq)]
pub struct TodoListComponentProps {
    todos: Vec<Todo>,
}

pub fn TodoListComponent(cx: Scope<TodoListComponentProps>) -> Element {
    cx.render(rsx! {
        span {
            id: "todo-list",
            for todo in cx.props.todos.clone() {
                TodoItemComponent { todo: todo }
            }
        }
    })
}

#[derive(Props, PartialEq, Eq)]
pub struct TodoCounterComponentProps {
    filter: TodoListFilter,
    num_items: u32,
}

pub fn TodoCounterComponent(cx: Scope<TodoCounterComponentProps>) -> Element {
    cx.render(rsx! {
        span {
            id: "todo-counter-{cx.props.filter}",
            class: "tag is-rounded todo-counter",
            "hx-swap-oob": true,
            "{cx.props.num_items}"
        }
    })
}

#[derive(Props, PartialEq, Eq)]
pub struct TodoTabsComponentProps {
    num_completed_items: u32,
    num_active_items: u32,
    num_all_items: u32,
}

pub fn TodoTabsComponent(cx: Scope<TodoTabsComponentProps>) -> Element {
    cx.render(rsx! {
        TodoCounterComponent { filter: TodoListFilter::Completed, num_items: cx.props.num_completed_items }
        TodoCounterComponent { filter: TodoListFilter::Active, num_items: cx.props.num_active_items }
        TodoCounterComponent { filter: TodoListFilter::All, num_items: cx.props.num_all_items }
    })
}

#[derive(Props, PartialEq, Eq)]
pub struct TodoDeleteCompletedComponentProps {
    is_disabled: bool,
}

pub fn TodoDeleteCompletedComponent(cx: Scope<TodoDeleteCompletedComponentProps>) -> Element {
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

#[derive(Props, PartialEq, Eq)]
pub struct TodoToggleCompletedComponentProps {
    is_disabled: bool,
    action: TodoToggleAction,
}

pub fn TodoToggleCompletedComponent(cx: Scope<TodoToggleCompletedComponentProps>) -> Element {
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
