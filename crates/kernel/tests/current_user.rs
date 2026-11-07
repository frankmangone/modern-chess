//! The `CurrentUser` extractor, mounted on a tiny router the way a real app uses it.

use axum::body::{to_bytes, Body};
use axum::extract::FromRef;
use axum::http::{header, Request, StatusCode};
use axum::routing::get;
use axum::Router;
use chrono::{Duration, Utc};
use modern_chess_kernel::{CurrentUser, TokenVerifier, UserId};
use modern_chess_testkeys::TestKeys;
use serde_json::{json, Value};
use tower::ServiceExt;

#[derive(Clone)]
struct State {
    verifier: TokenVerifier,
}

impl FromRef<State> for TokenVerifier {
    fn from_ref(state: &State) -> TokenVerifier {
        state.verifier.clone()
    }
}

fn app(keys: &TestKeys) -> Router {
    Router::new()
        .route(
            "/whoami",
            get(|user: CurrentUser| async move { user.handle.to_string() }),
        )
        .with_state(State {
            verifier: TokenVerifier::from_public_pem(&keys.public_pem).unwrap(),
        })
}

async fn call(app: &Router, authorization: Option<&str>) -> (StatusCode, Option<String>, String) {
    let mut request = Request::builder().uri("/whoami");
    if let Some(value) = authorization {
        request = request.header(header::AUTHORIZATION, value);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let challenge = response
        .headers()
        .get(header::WWW_AUTHENTICATE)
        .map(|v| v.to_str().unwrap().to_string());
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, challenge, String::from_utf8(body.to_vec()).unwrap())
}

fn valid(keys: &TestKeys) -> String {
    keys.token_for(
        UserId::new(),
        "Guest-1234",
        (Utc::now() + Duration::days(1)).timestamp(),
    )
}

#[tokio::test]
async fn a_valid_bearer_token_names_the_caller() {
    let keys = TestKeys::generate();
    let app = app(&keys);

    let (status, _, body) = call(&app, Some(&format!("Bearer {}", valid(&keys)))).await;
    let (lower_status, _, _) = call(&app, Some(&format!("bearer {}", valid(&keys)))).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "Guest-1234");
    assert_eq!(
        lower_status,
        StatusCode::OK,
        "the scheme name is case-insensitive"
    );
}

#[tokio::test]
async fn every_kind_of_bad_credential_is_401_with_a_plain_body() {
    let keys = TestKeys::generate();
    let other = TestKeys::generate();
    let expired = keys.token_for(
        UserId::new(),
        "Guest-1234",
        (Utc::now() - Duration::days(1)).timestamp(),
    );
    let wrong_key = valid(&other);
    let tampered = format!("{}x", valid(&keys));
    let app = app(&keys);

    let bad = [
        None,
        Some("".to_string()),
        Some("Bearer".to_string()),
        Some("Bearer ".to_string()),
        Some("Bearer garbage".to_string()),
        Some(format!("Basic {}", valid(&keys))),
        Some(valid(&keys)),
        Some(format!("Bearer {expired}")),
        Some(format!("Bearer {wrong_key}")),
        Some(format!("Bearer {tampered}")),
    ];

    for header_value in bad {
        let (status, challenge, body) = call(&app, header_value.as_deref()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{header_value:?}");
        assert_eq!(challenge.as_deref(), Some("Bearer"), "{header_value:?}");
        assert_eq!(
            serde_json::from_str::<Value>(&body).unwrap(),
            json!({ "error": "sign in required" }),
            "{header_value:?}"
        );
    }
}
