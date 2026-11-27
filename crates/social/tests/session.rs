//! Exchanging a provider's token for ours, and deleting an account, over HTTP against a real
//! Postgres (`make db-up`).

mod support;

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use chrono::{Duration, Utc};
use jsonwebtoken::jwk::JwkSet;
use modern_chess_kernel::TokenVerifier;
use modern_chess_social::provider::{
    KeyFuture, KeySource, ProviderConfig, ProviderError, ProviderVerifier,
};
use modern_chess_social::{router, SocialState, TokenIssuer};
use modern_chess_testdb::with_db;
use modern_chess_testkeys::TestKeys;
use serde_json::{json, Value};
use sqlx::PgPool;
use support::{claims, jwks, keys, CountingSource, ISSUER, PROVIDER};
use tower::ServiceExt;

fn provider_config() -> ProviderConfig {
    ProviderConfig {
        name: PROVIDER.to_string(),
        issuer: ISSUER.to_string(),
        jwks_url: format!("{ISSUER}/.well-known/jwks.json"),
        authorized_parties: Vec::new(),
    }
}

fn working_provider() -> Arc<ProviderVerifier> {
    let source = Arc::new(CountingSource::new(jwks(&[keys(0)])));
    Arc::new(ProviderVerifier::new(provider_config(), source))
}

struct BrokenSource;

impl KeySource for BrokenSource {
    fn fetch(&self) -> KeyFuture<'_> {
        Box::pin(async {
            Err::<JwkSet, _>(ProviderError::Keys(
                "connection refused at SECRET-INTERNAL-ADDRESS".to_string(),
            ))
        })
    }
}

async fn app(pool: &PgPool, ours: &TestKeys, provider: Option<Arc<ProviderVerifier>>) -> Router {
    modern_chess_social::migrate(pool).await.unwrap();
    let issuer = TokenIssuer::from_private_pem(&ours.private_pem).unwrap();
    let mut state = SocialState::new(pool.clone(), Arc::new(issuer));
    if let Some(provider) = provider {
        state = state.with_provider(provider);
    }
    router(state)
}

async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    bearer: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = bearer {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = match body {
        Some(json) => {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn provider_token(subject: &str) -> String {
    keys(0).sign(&claims(subject))
}

async fn exchange(app: &Router, bearer: Option<&str>, guest: Option<&str>) -> (StatusCode, Value) {
    let body = match guest {
        Some(token) => json!({ "guest_token": token }),
        None => json!({}),
    };
    send(app, Method::POST, "/session", bearer, Some(body)).await
}

#[tokio::test]
async fn a_provider_token_is_exchanged_for_our_own_token_and_a_user() {
    with_db(|pool| async move {
        let ours = TestKeys::generate();
        let app = app(&pool, &ours, Some(working_provider())).await;

        let (status, body) = exchange(&app, Some(&provider_token("user_1")), None).await;

        assert_eq!(status, StatusCode::OK);
        let handle = body["user"]["handle"].as_str().unwrap();
        assert!(handle.starts_with("Player-"), "{handle}");
        let claims = TokenVerifier::from_public_pem(&ours.public_pem)
            .unwrap()
            .verify(body["token"].as_str().unwrap(), Utc::now())
            .unwrap();
        assert_eq!(claims.sub.to_string(), body["user"]["id"].as_str().unwrap());
        assert!(
            !body.to_string().contains("example.test"),
            "the email must never be sent back: {body}"
        );
        let (status, _) = send(&app, Method::GET, "/me", body["token"].as_str(), None).await;
        assert_eq!(status, StatusCode::OK);
    })
    .await;
}

#[tokio::test]
async fn signing_in_twice_is_the_same_user() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;

        let (_, first) = exchange(&app, Some(&provider_token("user_1")), None).await;
        let (_, second) = exchange(&app, Some(&provider_token("user_1")), None).await;

        assert_eq!(first["user"]["id"], second["user"]["id"]);
    })
    .await;
}

#[tokio::test]
async fn a_guest_who_signs_in_keeps_their_user() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;
        let (_, guest) = send(&app, Method::POST, "/guests", None, None).await;

        let (status, body) = exchange(
            &app,
            Some(&provider_token("user_1")),
            guest["token"].as_str(),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["user"]["id"], guest["user"]["id"]);
        assert_eq!(body["user"]["handle"], guest["user"]["handle"]);
        assert_eq!(
            body["user"]["setup_pending"], true,
            "an account has a set-up"
        );
        assert_eq!(guest["user"]["setup_pending"], false, "a guest has none");
    })
    .await;
}

#[tokio::test]
async fn a_guest_token_that_is_not_good_is_ignored_not_an_error() {
    with_db(|pool| async move {
        let ours = TestKeys::generate();
        let app = app(&pool, &ours, Some(working_provider())).await;
        let expired = ours.token_for(
            modern_chess_kernel::UserId::new(),
            "Guest-1",
            (Utc::now() - Duration::days(1)).timestamp(),
        );

        for guest in ["garbage", expired.as_str()] {
            let (status, body) = exchange(&app, Some(&provider_token("user_1")), Some(guest)).await;

            assert_eq!(status, StatusCode::OK, "{guest}");
            assert!(body["user"]["handle"]
                .as_str()
                .unwrap()
                .starts_with("Player-"));
        }
    })
    .await;
}

#[tokio::test]
async fn credentials_that_are_not_a_good_provider_token_are_refused() {
    with_db(|pool| async move {
        let ours = TestKeys::generate();
        let app = app(&pool, &ours, Some(working_provider())).await;
        let mut expired = claims("user_1");
        expired["exp"] = json!(Utc::now().timestamp() - 600);
        let (_, guest) = send(&app, Method::POST, "/guests", None, None).await;

        let attempts = [
            None,
            Some("garbage".to_string()),
            Some(keys(0).sign(&expired)),
            Some(keys(2).sign(&claims("user_1"))),
            // One of our own tokens is not a provider token.
            Some(guest["token"].as_str().unwrap().to_string()),
        ];
        for bearer in attempts {
            let (status, body) = exchange(&app, bearer.as_deref(), None).await;

            assert_eq!(status, StatusCode::UNAUTHORIZED, "{bearer:?}");
            assert_eq!(body, json!({ "error": "sign in required" }));
        }
    })
    .await;
}

#[tokio::test]
async fn without_a_provider_configured_sign_in_is_unavailable() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), None).await;

        let (status, _) = exchange(&app, Some(&provider_token("user_1")), None).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    })
    .await;
}

#[tokio::test]
async fn when_the_providers_keys_cannot_be_loaded_the_caller_learns_nothing_about_why() {
    with_db(|pool| async move {
        let provider = Arc::new(ProviderVerifier::new(
            provider_config(),
            Arc::new(BrokenSource),
        ));
        let app = app(&pool, &TestKeys::generate(), Some(provider)).await;

        let (status, body) = exchange(&app, Some(&provider_token("user_1")), None).await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(
            !body.to_string().contains("SECRET-INTERNAL-ADDRESS"),
            "{body}"
        );
    })
    .await;
}

#[tokio::test]
async fn deleting_an_account_takes_a_fresh_provider_token_and_ends_the_account() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;
        let (_, signed_in) = exchange(&app, Some(&provider_token("user_1")), None).await;
        let token = signed_in["token"].as_str().unwrap();

        let (status, _) = send(
            &app,
            Method::DELETE,
            "/me",
            Some(&provider_token("user_1")),
            None,
        )
        .await;
        let (me, _) = send(&app, Method::GET, "/me", Some(token), None).await;
        let (again, _) = send(
            &app,
            Method::DELETE,
            "/me",
            Some(&provider_token("user_1")),
            None,
        )
        .await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(
            me,
            StatusCode::UNAUTHORIZED,
            "our old token stops working here"
        );
        assert_eq!(again, StatusCode::UNAUTHORIZED, "nothing left to delete");
    })
    .await;
}

#[tokio::test]
async fn our_own_token_cannot_delete_an_account_however_valid_it_is() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;
        let (_, signed_in) = exchange(&app, Some(&provider_token("user_1")), None).await;
        let token = signed_in["token"].as_str().unwrap();

        let (status, _) = send(&app, Method::DELETE, "/me", Some(token), None).await;
        let (me, _) = send(&app, Method::GET, "/me", Some(token), None).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(me, StatusCode::OK, "the account must still be there");
    })
    .await;
}

#[tokio::test]
async fn a_leftover_guest_token_cannot_delete_the_account_the_guest_became() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;
        let (_, guest) = send(&app, Method::POST, "/guests", None, None).await;
        let guest_token = guest["token"].as_str().unwrap();
        exchange(&app, Some(&provider_token("user_1")), Some(guest_token)).await;

        let (status, _) = send(&app, Method::DELETE, "/me", Some(guest_token), None).await;
        let (me, _) = send(&app, Method::GET, "/me", Some(guest_token), None).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(me, StatusCode::OK);
    })
    .await;
}

#[tokio::test]
async fn one_accounts_token_cannot_delete_another_account() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;
        let (_, one) = exchange(&app, Some(&provider_token("user_1")), None).await;
        let (_, two) = exchange(&app, Some(&provider_token("user_2")), None).await;

        let (status, _) = send(
            &app,
            Method::DELETE,
            "/me",
            Some(&provider_token("user_2")),
            None,
        )
        .await;
        let (first, _) = send(&app, Method::GET, "/me", one["token"].as_str(), None).await;
        let (second, _) = send(&app, Method::GET, "/me", two["token"].as_str(), None).await;

        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(first, StatusCode::OK);
        assert_eq!(second, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn deleting_refuses_missing_expired_or_unknown_provider_tokens() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;
        exchange(&app, Some(&provider_token("user_1")), None).await;
        let mut expired = claims("user_1");
        expired["exp"] = json!(Utc::now().timestamp() - 600);

        // A valid token for someone who never signed in here has no account to delete either.
        let stranger = provider_token("user_9");
        let attempts = [
            None,
            Some("garbage".to_string()),
            Some(keys(0).sign(&expired)),
            Some(stranger),
        ];
        for bearer in attempts {
            let (status, _) = send(&app, Method::DELETE, "/me", bearer.as_deref(), None).await;

            assert_eq!(status, StatusCode::UNAUTHORIZED, "{bearer:?}");
        }
    })
    .await;
}

#[tokio::test]
async fn deleting_without_a_provider_configured_is_unavailable() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), None).await;

        let (status, _) = send(&app, Method::DELETE, "/me", Some("anything"), None).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    })
    .await;
}

#[tokio::test]
async fn the_body_is_optional() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;
        let (status, body) = send(
            &app,
            Method::POST,
            "/session",
            Some(&provider_token("user_1")),
            None,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert!(body["user"]["handle"]
            .as_str()
            .unwrap()
            .starts_with("Player-"));
    })
    .await;
}

#[tokio::test]
async fn without_a_provider_a_request_with_no_body_still_says_sign_in_is_unavailable() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), None).await;

        let (status, _) = send(&app, Method::POST, "/session", None, None).await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    })
    .await;
}

// ---- The set-up a new account goes through ----

/// Signs in `subject` as a new account and returns its token.
async fn account(app: &Router, subject: &str) -> (String, Value) {
    let (status, body) = exchange(app, Some(&provider_token(subject)), None).await;
    assert_eq!(status, StatusCode::OK);
    (
        body["token"].as_str().unwrap().to_string(),
        body["user"].clone(),
    )
}

async fn put_handle(app: &Router, token: &str, handle: &str) -> (StatusCode, Value) {
    send(
        app,
        Method::PUT,
        "/me/handle",
        Some(token),
        Some(json!({ "handle": handle })),
    )
    .await
}

#[tokio::test]
async fn a_new_account_chooses_a_username_and_gets_a_token_that_carries_it() {
    with_db(|pool| async move {
        let ours = TestKeys::generate();
        let app = app(&pool, &ours, Some(working_provider())).await;
        let (token, user) = account(&app, "user_1").await;
        assert_eq!(user["setup_pending"], true);

        let (status, body) = put_handle(&app, &token, "Ada_L").await;

        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["user"]["handle"], "Ada_L");
        let verifier = TokenVerifier::from_public_pem(&ours.public_pem).unwrap();
        let claims = verifier
            .verify(body["token"].as_str().unwrap(), Utc::now())
            .unwrap();
        assert_eq!(claims.handle.as_str(), "Ada_L");
        let (_, me) = send(&app, Method::GET, "/me", Some(&token), None).await;
        assert_eq!(
            me["user"]["handle"], "Ada_L",
            "the old token still works and shows the new name"
        );
    })
    .await;
}

#[tokio::test]
async fn a_username_that_is_not_allowed_is_refused_with_a_reason() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;
        let (token, user) = account(&app, "user_1").await;

        for (name, code) in [
            ("ab", "too_short"),
            ("has space", "bad_characters"),
            ("Admin", "reserved"),
            ("Guest-7", "generated"),
        ] {
            let (status, body) = put_handle(&app, &token, name).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{name}");
            assert_eq!(body["code"], code, "{name}");
            assert!(body["error"].as_str().unwrap().ends_with('.'));
        }
        let (_, me) = send(&app, Method::GET, "/me", Some(&token), None).await;
        assert_eq!(me["user"]["handle"], user["handle"], "nothing changed");
    })
    .await;
}

#[tokio::test]
async fn a_username_someone_has_is_a_conflict_in_any_letter_case() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;
        let (ada, _) = account(&app, "user_1").await;
        let (bob, _) = account(&app, "user_2").await;
        assert_eq!(put_handle(&app, &ada, "Ada").await.0, StatusCode::OK);

        let (status, body) = put_handle(&app, &bob, "aDA").await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["code"], "taken");
    })
    .await;
}

#[tokio::test]
async fn a_name_can_be_checked_before_it_is_chosen() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;
        let (ada, _) = account(&app, "user_1").await;
        let (bob, _) = account(&app, "user_2").await;
        put_handle(&app, &ada, "Ada").await;

        let (_, free) = send(&app, Method::GET, "/handles/Grace", Some(&bob), None).await;
        let (_, taken) = send(&app, Method::GET, "/handles/ADA", Some(&bob), None).await;
        let (_, bad) = send(&app, Method::GET, "/handles/x", Some(&bob), None).await;

        assert_eq!(free, json!({ "available": true, "problem": null }));
        assert_eq!(taken["available"], false);
        assert_eq!(taken["problem"]["code"], "taken");
        assert_eq!(bad["problem"]["code"], "too_short");
    })
    .await;
}

#[tokio::test]
async fn checking_a_name_needs_a_token() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;

        let (status, _) = send(&app, Method::GET, "/handles/Grace", None, None).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn finishing_the_set_up_ends_it_and_closes_the_door_on_renaming() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;
        let (token, _) = account(&app, "user_1").await;
        put_handle(&app, &token, "Ada").await;

        let (status, body) = send(&app, Method::POST, "/me/setup", Some(&token), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["user"]["setup_pending"], false);
        let (_, again) = send(&app, Method::POST, "/me/setup", Some(&token), None).await;
        assert_eq!(
            again["user"]["setup_pending"], false,
            "asking twice is harmless"
        );

        let (status, body) = put_handle(&app, &token, "Grace").await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["code"], "no_setup");
        let (_, me) = send(&app, Method::GET, "/me", Some(&token), None).await;
        assert_eq!(me["user"]["handle"], "Ada");
        assert_eq!(me["user"]["setup_pending"], false);
    })
    .await;
}

#[tokio::test]
async fn a_guest_has_no_set_up() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;
        let (_, guest) = send(&app, Method::POST, "/guests", None, None).await;
        let token = guest["token"].as_str().unwrap();

        let (status, body) = put_handle(&app, token, "Sneaky").await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["code"], "no_setup");
    })
    .await;
}

#[tokio::test]
async fn the_set_up_needs_a_token() {
    with_db(|pool| async move {
        let app = app(&pool, &TestKeys::generate(), Some(working_provider())).await;

        let (put, _) = send(
            &app,
            Method::PUT,
            "/me/handle",
            None,
            Some(json!({ "handle": "Ada" })),
        )
        .await;
        let (post, _) = send(&app, Method::POST, "/me/setup", None, None).await;

        assert_eq!(
            (put, post),
            (StatusCode::UNAUTHORIZED, StatusCode::UNAUTHORIZED)
        );
    })
    .await;
}
