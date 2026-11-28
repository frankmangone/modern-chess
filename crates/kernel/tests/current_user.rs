//! The `CurrentUser` extractor, mounted on a tiny router the way a real app uses it.

use axum::body::{to_bytes, Body};
use axum::extract::FromRef;
use axum::http::{header, Request, StatusCode};
use axum::routing::get;
use axum::Router;
use chrono::{Duration, Utc};
use modern_chess_kernel::{AccountUser, CurrentUser, TokenVerifier, UserId};
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
        .route(
            "/account",
            get(|user: AccountUser| async move { user.handle.to_string() }),
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

async fn call_account(app: &Router, authorization: Option<&str>) -> (StatusCode, String) {
    let mut request = Request::builder().uri("/account");
    if let Some(value) = authorization {
        request = request.header(header::AUTHORIZATION, value);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn an_account_token_opens_what_asks_for_an_account() {
    let keys = TestKeys::generate();
    let exp = (Utc::now() + Duration::days(1)).timestamp();
    let token = keys.account_token_for(UserId::new(), "Ada", exp);

    let (status, body) = call_account(&app(&keys), Some(&format!("Bearer {token}"))).await;

    assert_eq!((status, body.as_str()), (StatusCode::OK, "Ada"));
}

#[tokio::test]
async fn a_guest_token_is_refused_with_a_code_that_says_why() {
    let keys = TestKeys::generate();

    let (status, body) = call_account(&app(&keys), Some(&format!("Bearer {}", valid(&keys)))).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["code"], "account_required");
}

#[tokio::test]
async fn a_token_made_before_the_claim_existed_is_a_guest_token_here() {
    let keys = TestKeys::generate();
    let old = keys.sign(&json!({
        "sub": UserId::new(),
        "handle": "Ada",
        "exp": (Utc::now() + Duration::days(1)).timestamp(),
    }));

    let (status, _) = call_account(&app(&keys), Some(&format!("Bearer {old}"))).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn no_token_or_a_bad_one_is_a_401_not_a_403() {
    let keys = TestKeys::generate();
    let other = TestKeys::generate();
    let forged = other.account_token_for(
        UserId::new(),
        "Ada",
        (Utc::now() + Duration::days(1)).timestamp(),
    );

    assert_eq!(
        call_account(&app(&keys), None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call_account(&app(&keys), Some(&format!("Bearer {forged}")))
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}
