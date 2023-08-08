use axum::{
    body::{Body, HttpBody},
    http::{Request, Response, StatusCode},
};
use scraper::{Html, Selector};
use std::fmt::Debug;
use todomvc::{
    app,
    models::{TodoListFilter, TodoToggleAction},
    SharedState,
};
use tower::ServiceExt;

async fn parse_response_body<T: HttpBody>(response: Response<T>) -> String
where
    <T as HttpBody>::Error: Debug,
{
    let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

#[tokio::test]
async fn test_list_todo_empty() {
    // Arrange
    let shared_state = SharedState::default();
    let local_state = shared_state.clone();
    let app = app(shared_state);
    let request = Request::get("/todo?filter=All")
        .body(Body::empty())
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        local_state.read().unwrap().selected_filter,
        TodoListFilter::All
    );

    let body = parse_response_body(response).await;
    let document = Html::parse_document(&body);

    let counter_selector = Selector::parse(".todo-counter").unwrap();
    let toggle_selector = Selector::parse("#todo-toggle-completed").unwrap();
    let list_selector = Selector::parse("#todo-list .todo-item p").unwrap();

    assert_eq!(document.select(&counter_selector).count(), 3);
    assert_eq!(document.select(&toggle_selector).count(), 1);
    assert_eq!(document.select(&list_selector).count(), 0);

    assert!(document
        .select(&toggle_selector)
        .next()
        .unwrap()
        .value()
        .attr("disabled")
        .is_some());
}

#[tokio::test]
async fn test_list_todo_non_empty() {
    // Arrange
    let shared_state = SharedState::default();
    let local_state = shared_state.clone();

    {
        let todo_repo = &mut shared_state.write().unwrap().todo_repo;

        todo_repo.create("a");
        todo_repo.create("b");
        todo_repo.create("c");
    }

    let app = app(shared_state);
    let request = Request::get("/todo?filter=Active")
        .body(Body::empty())
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        local_state.read().unwrap().selected_filter,
        TodoListFilter::Active
    );

    let body = parse_response_body(response).await;
    let document = Html::parse_document(&body);

    let counter_selector = Selector::parse(".todo-counter").unwrap();
    let toggle_selector = Selector::parse("#todo-toggle-completed").unwrap();
    let list_selector = Selector::parse("#todo-list .todo-item p").unwrap();

    assert_eq!(document.select(&counter_selector).count(), 3);
    assert_eq!(document.select(&toggle_selector).count(), 1);
    assert_eq!(document.select(&list_selector).count(), 3);

    assert!(document
        .select(&toggle_selector)
        .next()
        .unwrap()
        .value()
        .attr("disabled")
        .is_none());
}

#[tokio::test]
async fn test_create_todo() {
    // Arrange
    let shared_state = SharedState::default();
    let local_state = shared_state.clone();

    let app = app(shared_state);
    let request = Request::post("/todo")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::from("text=a"))
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        local_state.read().unwrap().toggle_action,
        TodoToggleAction::Check
    );

    let body = parse_response_body(response).await;
    let document = Html::parse_document(&body);

    let counter_selector = Selector::parse(".todo-counter").unwrap();
    let toggle_selector = Selector::parse("#todo-toggle-completed").unwrap();
    let item_selector = Selector::parse(".todo-item p").unwrap();

    assert_eq!(document.select(&counter_selector).count(), 3);
    assert_eq!(document.select(&toggle_selector).count(), 1);
    assert_eq!(document.select(&item_selector).count(), 1);

    assert_eq!(
        document.select(&item_selector).next().unwrap().inner_html(),
        "a"
    );
    assert!(document
        .select(&toggle_selector)
        .next()
        .unwrap()
        .value()
        .attr("disabled")
        .is_none());
}

#[tokio::test]
async fn test_toggle_completed_todo() {
    // Arrange
    let shared_state = SharedState::default();
    let local_state = shared_state.clone();

    {
        let todo_repo = &mut shared_state.write().unwrap().todo_repo;

        todo_repo.create("a");
        todo_repo.create("b");
        todo_repo.create("c");
    }

    let app = app(shared_state);
    let request = Request::patch("/todo?action=Check")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::empty())
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        local_state.read().unwrap().toggle_action,
        TodoToggleAction::Uncheck
    );

    let body = parse_response_body(response).await;
    let document = Html::parse_document(&body);

    let counter_selector = Selector::parse(".todo-counter").unwrap();
    let delete_selector = Selector::parse("#todo-delete-completed").unwrap();
    let toggle_selector = Selector::parse("#todo-toggle-completed").unwrap();
    let item_selector = Selector::parse(".todo-item input").unwrap();

    assert_eq!(document.select(&counter_selector).count(), 3);
    assert_eq!(document.select(&delete_selector).count(), 1);
    assert_eq!(document.select(&toggle_selector).count(), 1);
    assert_eq!(document.select(&item_selector).count(), 3);

    assert_eq!(
        document
            .select(&item_selector)
            .filter(|e| e.value().attr("checked").is_some())
            .count(),
        3
    );
    assert!(document
        .select(&toggle_selector)
        .next()
        .unwrap()
        .value()
        .attr("disabled")
        .is_none());
    assert!(document
        .select(&delete_selector)
        .next()
        .unwrap()
        .value()
        .attr("disabled")
        .is_none());
}

#[tokio::test]
async fn test_delete_completed_todo() {
    // Arrange
    let shared_state = SharedState::default();
    let local_state = shared_state.clone();

    {
        let todo_repo = &mut shared_state.write().unwrap().todo_repo;

        todo_repo.create("a");
        todo_repo.create("b");
        todo_repo.toggle_completed(&TodoToggleAction::Check);
        todo_repo.create("c");
    }

    let app = app(shared_state);
    let request = Request::delete("/todo").body(Body::empty()).unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        local_state.read().unwrap().toggle_action,
        TodoToggleAction::Check
    );

    let body = parse_response_body(response).await;
    let document = Html::parse_document(&body);

    let counter_selector = Selector::parse(".todo-counter").unwrap();
    let delete_selector = Selector::parse("#todo-delete-completed").unwrap();
    let toggle_selector = Selector::parse("#todo-toggle-completed").unwrap();
    let item_selector = Selector::parse(".todo-item p").unwrap();

    assert_eq!(document.select(&counter_selector).count(), 3);
    assert_eq!(document.select(&delete_selector).count(), 1);
    assert_eq!(document.select(&toggle_selector).count(), 1);
    assert_eq!(document.select(&item_selector).count(), 1);

    assert_eq!(
        document.select(&item_selector).next().unwrap().inner_html(),
        "c"
    );
    assert!(document
        .select(&toggle_selector)
        .next()
        .unwrap()
        .value()
        .attr("disabled")
        .is_none());
    assert!(document
        .select(&delete_selector)
        .next()
        .unwrap()
        .value()
        .attr("disabled")
        .is_some());
}

#[tokio::test]
async fn test_edit_todo() {
    // Arrange
    let shared_state = SharedState::default();
    let id;

    {
        let todo_repo = &mut shared_state.write().unwrap().todo_repo;
        let todo = todo_repo.create("a");

        id = todo.id;
    }

    let app = app(shared_state);
    let request = Request::get(format!("/todo/{id}"))
        .body(Body::empty())
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    let body = parse_response_body(response).await;
    let document = Html::parse_document(&body);
    let edit_selector = Selector::parse(".todo-edit p input").unwrap();

    assert_eq!(document.select(&edit_selector).count(), 1);
    assert_eq!(
        document
            .select(&edit_selector)
            .next()
            .unwrap()
            .value()
            .attr("value"),
        Some("a")
    );
}

#[tokio::test]
async fn test_update_todo() {
    // Arrange
    let shared_state = SharedState::default();
    let id;

    {
        let todo_repo = &mut shared_state.write().unwrap().todo_repo;
        let todo = todo_repo.create("a");

        id = todo.id;
    }

    let app = app(shared_state);
    let request = Request::patch(format!("/todo/{id}"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::from("text=b&is_completed=true"))
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    let body = parse_response_body(response).await;
    let document = Html::parse_document(&body);

    let item_input_selector = Selector::parse(".todo-item input").unwrap();
    let counter_selector = Selector::parse(".todo-counter").unwrap();
    let delete_selector = Selector::parse("#todo-delete-completed").unwrap();
    let toggle_selector = Selector::parse("#todo-toggle-completed").unwrap();
    let item_p_selector = Selector::parse(".todo-item p s").unwrap();

    assert_eq!(document.select(&counter_selector).count(), 3);
    assert_eq!(document.select(&delete_selector).count(), 1);
    assert_eq!(document.select(&toggle_selector).count(), 1);
    assert_eq!(document.select(&item_p_selector).count(), 1);

    assert_eq!(
        document
            .select(&item_p_selector)
            .next()
            .unwrap()
            .inner_html(),
        "b"
    );
    assert_eq!(
        document
            .select(&toggle_selector)
            .next()
            .unwrap()
            .inner_html(),
        "Uncheck all"
    );
    assert!(document
        .select(&item_input_selector)
        .next()
        .unwrap()
        .value()
        .attr("checked")
        .is_some());
    assert!(document
        .select(&delete_selector)
        .next()
        .unwrap()
        .value()
        .attr("disabled")
        .is_none());
}

#[tokio::test]
async fn test_delete_todo() {
    // Arrange
    let shared_state = SharedState::default();
    let id;

    {
        let todo_repo = &mut shared_state.write().unwrap().todo_repo;
        let todo = todo_repo.create("a");

        id = todo.id;
    }

    let app = app(shared_state);
    let request = Request::delete(format!("/todo/{id}"))
        .body(Body::empty())
        .unwrap();

    // Act
    let response = app.oneshot(request).await.unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    let body = parse_response_body(response).await;
    let document = Html::parse_document(&body);

    let counter_selector = Selector::parse(".todo-counter").unwrap();
    let delete_selector = Selector::parse("#todo-delete-completed").unwrap();
    let toggle_selector = Selector::parse("#todo-toggle-completed").unwrap();
    let item_selector = Selector::parse(".todo-item").unwrap();

    assert_eq!(document.select(&counter_selector).count(), 3);
    assert_eq!(document.select(&delete_selector).count(), 1);
    assert_eq!(document.select(&toggle_selector).count(), 1);
    assert_eq!(document.select(&item_selector).count(), 0);

    assert_eq!(
        document
            .select(&toggle_selector)
            .next()
            .unwrap()
            .inner_html(),
        "Check all"
    );
    assert!(document
        .select(&delete_selector)
        .next()
        .unwrap()
        .value()
        .attr("disabled")
        .is_some());
    assert!(document
        .select(&delete_selector)
        .next()
        .unwrap()
        .value()
        .attr("disabled")
        .is_some());
}
