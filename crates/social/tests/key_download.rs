//! Downloading a provider's key set over HTTP, from a small local server.

mod support;

use std::net::SocketAddr;

use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use chrono::Utc;
use modern_chess_social::provider::{HttpKeySource, KeySource, ProviderConfig, ProviderVerifier};
use support::{claims, jwks, keys, ISSUER, PROVIDER};
use tokio::net::TcpListener;

async fn serve(app: Router) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    addr
}

fn keys_json() -> String {
    serde_json::to_string(&jwks(&[keys(0)])).unwrap()
}

#[tokio::test]
async fn the_key_set_is_downloaded_and_tokens_verify_against_it() {
    let body = keys_json();
    let addr = serve(Router::new().route("/jwks", get(move || async move { body.clone() }))).await;
    let config = ProviderConfig {
        name: PROVIDER.to_string(),
        issuer: ISSUER.to_string(),
        jwks_url: format!("http://{addr}/jwks"),
        authorized_parties: Vec::new(),
    };
    let verifier = ProviderVerifier::connect(config).unwrap();

    let identity = verifier
        .verify(&keys(0).sign(&claims("user_1")), Utc::now())
        .await
        .unwrap();

    assert_eq!(identity.subject, "user_1");
}

#[tokio::test]
async fn an_error_status_or_a_body_that_is_not_a_key_set_is_a_failure() {
    let addr = serve(
        Router::new()
            .route("/down", get(|| async { StatusCode::BAD_GATEWAY }))
            .route("/junk", get(|| async { "<html>not json</html>" }))
            .route("/huge", get(|| async { "x".repeat(2 * 1024 * 1024) })),
    )
    .await;

    for path in ["down", "junk", "huge", "missing"] {
        let source = HttpKeySource::new(&format!("http://{addr}/{path}")).unwrap();

        assert!(source.fetch().await.is_err(), "{path}");
    }
}

#[tokio::test]
async fn an_unreachable_provider_is_a_failure_not_a_hang() {
    let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = closed.local_addr().unwrap();
    drop(closed);
    let source = HttpKeySource::new(&format!("http://{addr}/jwks")).unwrap();

    assert!(source.fetch().await.is_err());
}
