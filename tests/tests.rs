use axum::{
    body::Body,
    http::{self, Request, StatusCode},
};
use todomvc::app;
use tower::Service;
use tower::ServiceExt;

#[tokio::test]
async fn hello_world() {
    let app = app();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
