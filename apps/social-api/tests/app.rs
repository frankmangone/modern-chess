//! The assembled social app: root health routes plus the social API under its prefix.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use modern_chess_social::{SocialState, TokenIssuer};
use modern_chess_social_api::app_router;
use modern_chess_testdb::with_db;
use modern_chess_testkeys::TestKeys;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

async fn app(pool: &PgPool) -> Router {
    modern_chess_social::migrate(pool).await.unwrap();
    let issuer = TokenIssuer::from_private_pem(&TestKeys::generate().private_pem).unwrap();
    let state = SocialState::new(pool.clone(), Arc::new(issuer));
    app_router(pool.clone(), state, false)
}

async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    bearer: Option<&str>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = bearer {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn health_and_ready_are_served_at_the_root() {
    with_db(|pool| async move {
        let app = app(&pool).await;

        let (health, _) = send(&app, Method::GET, "/health", None).await;
        let (ready, body) = send(&app, Method::GET, "/ready", None).await;

        assert_eq!(health, StatusCode::OK);
        assert_eq!(ready, StatusCode::OK);
        assert_eq!(body["status"], "ready");
    })
    .await;
}

#[tokio::test]
async fn ready_reports_degraded_when_the_database_is_gone() {
    with_db(|pool| async move {
        let app = app(&pool).await;
        pool.close().await;

        let (status, body) = send(&app, Method::GET, "/ready", None).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["status"], "degraded");
    })
    .await;
}

#[tokio::test]
async fn the_social_api_lives_under_its_prefix() {
    with_db(|pool| async move {
        let app = app(&pool).await;

        let (created, guest) = send(&app, Method::POST, "/api/social/guests", None).await;
        let token = guest["token"].as_str().unwrap();
        let (me_status, me) = send(&app, Method::GET, "/api/social/me", Some(token)).await;
        let (bare, _) = send(&app, Method::POST, "/guests", None).await;

        assert_eq!(created, StatusCode::CREATED);
        assert_eq!(me_status, StatusCode::OK);
        assert_eq!(me["user"], guest["user"]);
        assert_eq!(bare, StatusCode::NOT_FOUND, "no unprefixed routes");
    })
    .await;
}
