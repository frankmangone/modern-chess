//! A handler that panics must answer with a plain 500 and leave the server serving.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;
use modern_chess_social_api::guard_panics;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn boom() -> &'static str {
    panic!("index 9 out of range for the secret table")
}

fn app() -> Router {
    guard_panics(
        Router::new()
            .route("/boom", get(boom))
            .route("/fine", get(|| async { "fine" })),
    )
}

async fn get_status(app: &Router, uri: &str) -> (StatusCode, Vec<u8>) {
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, bytes.to_vec())
}

#[tokio::test]
async fn a_panicking_handler_answers_with_a_plain_500() {
    let (status, body) = get_status(&app(), "/boom").await;

    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json, json!({ "error": "something went wrong" }));
}

#[tokio::test]
async fn the_panic_message_is_never_sent_to_the_caller() {
    let (_, body) = get_status(&app(), "/boom").await;

    assert!(!String::from_utf8_lossy(&body).contains("secret"));
}

#[tokio::test]
async fn the_app_keeps_serving_after_a_panic() {
    let app = app();
    get_status(&app, "/boom").await;

    let (status, body) = get_status(&app, "/fine").await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"fine");
}
