//! Guest sign-up and "who am I" over HTTP, against a real Postgres (`make db-up`).

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use chrono::{DateTime, Duration, TimeZone, Utc};
use modern_chess_kernel::{TokenVerifier, UserId};
use modern_chess_social::guests::create_guest;
use modern_chess_social::store::UserRepo;
use modern_chess_social::{router, SocialState, TokenIssuer};
use modern_chess_testdb::with_db;
use modern_chess_testkeys::TestKeys;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap()
}

async fn app(pool: &PgPool, keys: &TestKeys) -> Router {
    modern_chess_social::migrate(pool).await.unwrap();
    let issuer = TokenIssuer::from_private_pem(&keys.private_pem).unwrap();
    router(SocialState::new(pool.clone(), Arc::new(issuer)))
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
async fn guest_signup_returns_a_user_and_a_token_that_verifies() {
    with_db(|pool| async move {
        let keys = TestKeys::generate();
        let app = app(&pool, &keys).await;

        let (status, body) = send(&app, Method::POST, "/guests", None).await;

        assert_eq!(status, StatusCode::CREATED);
        let handle = body["user"]["handle"].as_str().unwrap();
        assert!(handle.starts_with("Guest-"), "{handle}");
        let claims = TokenVerifier::from_public_pem(&keys.public_pem)
            .unwrap()
            .verify(body["token"].as_str().unwrap(), Utc::now())
            .unwrap();
        assert_eq!(claims.sub.to_string(), body["user"]["id"].as_str().unwrap());
        assert_eq!(claims.handle.as_str(), handle);
        let stored = UserRepo::new(pool.clone()).get(claims.sub).await.unwrap();
        assert_eq!(stored.unwrap().handle.as_str(), handle);
    })
    .await;
}

#[tokio::test]
async fn many_guests_all_get_different_handles() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate()).await;

        let mut handles = std::collections::HashSet::new();
        for _ in 0..25 {
            let (status, body) = send(&app, Method::POST, "/guests", None).await;
            assert_eq!(status, StatusCode::CREATED);
            handles.insert(body["user"]["handle"].as_str().unwrap().to_lowercase());
        }

        assert_eq!(handles.len(), 25);
    })
    .await;
}

#[tokio::test]
async fn me_returns_the_callers_user() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate()).await;
        let (_, created) = send(&app, Method::POST, "/guests", None).await;
        let token = created["token"].as_str().unwrap();

        let (status, me) = send(&app, Method::GET, "/me", Some(token)).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(me["user"], created["user"]);
    })
    .await;
}

#[tokio::test]
async fn me_without_a_valid_token_is_401() {
    with_db(|pool| async move {
        let keys = TestKeys::generate();
        let app = app(&pool, &keys).await;
        let (_, created) = send(&app, Method::POST, "/guests", None).await;
        let user: UserId = created["user"]["id"].as_str().unwrap().parse().unwrap();
        let other_key = TestKeys::generate();
        let expired = keys.token_for(
            user,
            "Guest-1234",
            (Utc::now() - Duration::hours(1)).timestamp(),
        );
        let wrong_key = other_key.token_for(
            user,
            "Guest-1234",
            (Utc::now() + Duration::hours(1)).timestamp(),
        );

        for token in [
            None,
            Some("garbage"),
            Some(expired.as_str()),
            Some(wrong_key.as_str()),
        ] {
            let (status, body) = send(&app, Method::GET, "/me", token).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{token:?}");
            assert_eq!(body, json!({ "error": "sign in required" }), "{token:?}");
        }
    })
    .await;
}

#[tokio::test]
async fn me_for_a_user_that_no_longer_exists_is_401() {
    with_db(|pool| async move {
        let keys = TestKeys::generate();
        let app = app(&pool, &keys).await;
        let ghost = keys.token_for(
            UserId::new(),
            "Guest-9999",
            (Utc::now() + Duration::hours(1)).timestamp(),
        );

        let (status, _) = send(&app, Method::GET, "/me", Some(&ghost)).await;

        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "the web app then signs in as a new guest"
        );
    })
    .await;
}

#[tokio::test]
async fn a_guest_handle_clash_is_retried_with_another_number() {
    with_db(|pool| async move {
        modern_chess_social::migrate(&pool).await.unwrap();
        let users = UserRepo::new(pool.clone());
        let mut numbers = vec![1234u32, 1234, 5678].into_iter();

        let first = create_guest(&users, t0(), |_| numbers.next().unwrap())
            .await
            .unwrap();
        let second = create_guest(&users, t0(), |_| numbers.next().unwrap())
            .await
            .unwrap();
        let third = create_guest(&users, t0(), |_| 1234).await;

        assert_eq!(first.handle.as_str(), "Guest-1234");
        assert_eq!(
            second.handle.as_str(),
            "Guest-5678",
            "the clash on 1234 was retried"
        );
        assert!(
            third.is_err(),
            "a generator that only repeats gives up instead of looping"
        );
    })
    .await;
}
