//! `Option<CurrentUser>`: for pages anyone may look at, where a signed-in caller just sees
//! a little more. A bad credential is the same as none, never an error.

use axum::body::{to_bytes, Body};
use axum::extract::FromRef;
use axum::http::{header, Request, StatusCode};
use axum::routing::get;
use axum::Router;
use chrono::{Duration, Utc};
use modern_chess_kernel::{CurrentUser, TokenVerifier, UserId};
use modern_chess_testkeys::TestKeys;
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

async fn ask(keys: &TestKeys, authorization: Option<&str>) -> (StatusCode, String) {
    let app = Router::new()
        .route(
            "/look",
            get(|user: Option<CurrentUser>| async move {
                user.map_or("nobody".to_string(), |u| u.handle.to_string())
            }),
        )
        .with_state(State {
            verifier: TokenVerifier::from_public_pem(&keys.public_pem).unwrap(),
        });
    let mut request = Request::builder().uri("/look");
    if let Some(value) = authorization {
        request = request.header(header::AUTHORIZATION, value);
    }
    let response = app
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

#[tokio::test]
async fn a_valid_token_names_the_caller_and_anything_else_is_nobody() {
    let keys = TestKeys::generate();
    let good = keys.token_for(
        UserId::new(),
        "Guest-1234",
        (Utc::now() + Duration::days(1)).timestamp(),
    );
    let expired = keys.token_for(
        UserId::new(),
        "Guest-1234",
        (Utc::now() - Duration::days(1)).timestamp(),
    );
    let foreign = TestKeys::generate().token_for(
        UserId::new(),
        "Guest-1234",
        (Utc::now() + Duration::days(1)).timestamp(),
    );

    assert_eq!(
        ask(&keys, Some(&format!("Bearer {good}"))).await,
        (StatusCode::OK, "Guest-1234".to_string())
    );
    for authorization in [
        None,
        Some("garbage".to_string()),
        Some(format!("Bearer {expired}")),
        Some(format!("Bearer {foreign}")),
        Some("Bearer ".to_string()),
    ] {
        assert_eq!(
            ask(&keys, authorization.as_deref()).await,
            (StatusCode::OK, "nobody".to_string()),
            "{authorization:?}"
        );
    }
}
