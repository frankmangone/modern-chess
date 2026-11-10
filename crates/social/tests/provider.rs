//! Checking a sign-in provider's token: signature, issuer, time, origin, and key rotation.

mod support;

use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::Utc;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use modern_chess_social::provider::{ProviderConfig, ProviderError, ProviderVerifier};
use serde_json::{json, Value};
use support::{claims, jwks, keys, CountingSource, ISSUER, PROVIDER};

fn config(authorized_parties: &[&str]) -> ProviderConfig {
    ProviderConfig {
        name: PROVIDER.to_string(),
        issuer: ISSUER.to_string(),
        jwks_url: format!("{ISSUER}/.well-known/jwks.json"),
        authorized_parties: authorized_parties.iter().map(|p| p.to_string()).collect(),
    }
}

fn verifier_with(source: Arc<CountingSource>, parties: &[&str]) -> ProviderVerifier {
    ProviderVerifier::new(config(parties), source)
}

fn verifier() -> ProviderVerifier {
    verifier_with(Arc::new(CountingSource::new(jwks(&[keys(0)]))), &[])
}

async fn accepts(verifier: &ProviderVerifier, token: &str) -> bool {
    verifier.verify(token, Utc::now()).await.is_ok()
}

#[tokio::test]
async fn a_genuine_token_names_the_provider_subject_and_verified_email() {
    let token = keys(0).sign(&claims("user_123"));

    let identity = verifier().verify(&token, Utc::now()).await.unwrap();

    assert_eq!(identity.provider, PROVIDER);
    assert_eq!(identity.subject, "user_123");
    assert_eq!(identity.email.as_deref(), Some("player@example.test"));
}

#[tokio::test]
async fn an_email_the_provider_has_not_verified_is_dropped() {
    let mut body = claims("user_123");
    body["email_verified"] = json!(false);
    let unverified = verifier()
        .verify(&keys(0).sign(&body), Utc::now())
        .await
        .unwrap();

    body.as_object_mut().unwrap().remove("email_verified");
    let unstated = verifier()
        .verify(&keys(0).sign(&body), Utc::now())
        .await
        .unwrap();

    assert_eq!(unverified.email, None);
    assert_eq!(unstated.email, None);
}

#[tokio::test]
async fn a_token_without_an_email_is_still_accepted() {
    let mut body = claims("user_123");
    body.as_object_mut().unwrap().remove("email");

    let identity = verifier()
        .verify(&keys(0).sign(&body), Utc::now())
        .await
        .unwrap();

    assert_eq!(identity.email, None);
}

#[tokio::test]
async fn an_expired_token_is_refused() {
    let mut body = claims("user_123");
    body["exp"] = json!(Utc::now().timestamp() - 60);

    assert!(!accepts(&verifier(), &keys(0).sign(&body)).await);
}

#[tokio::test]
async fn a_token_that_is_not_valid_yet_is_refused() {
    let mut body = claims("user_123");
    body["nbf"] = json!(Utc::now().timestamp() + 600);

    assert!(!accepts(&verifier(), &keys(0).sign(&body)).await);
}

#[tokio::test]
async fn a_token_from_another_issuer_is_refused() {
    let mut body = claims("user_123");
    body["iss"] = json!("https://elsewhere.example.test");

    assert!(!accepts(&verifier(), &keys(0).sign(&body)).await);
}

#[tokio::test]
async fn a_token_without_an_issuer_is_refused() {
    let mut body = claims("user_123");
    body.as_object_mut().unwrap().remove("iss");

    assert!(!accepts(&verifier(), &keys(0).sign(&body)).await);
}

#[tokio::test]
async fn a_token_without_a_subject_or_with_a_blank_or_huge_one_is_refused() {
    let mut missing = claims("x");
    missing.as_object_mut().unwrap().remove("sub");

    assert!(!accepts(&verifier(), &keys(0).sign(&missing)).await);
    assert!(!accepts(&verifier(), &keys(0).sign(&claims(""))).await);
    assert!(!accepts(&verifier(), &keys(0).sign(&claims(&"u".repeat(300)))).await);
}

/// A token with its header changed by `edit`, the signature left as it was.
fn edit_header(token: &str, edit: impl FnOnce(&mut Value)) -> String {
    let mut parts = token.split('.');
    let header = URL_SAFE_NO_PAD.decode(parts.next().unwrap()).unwrap();
    let mut header: Value = serde_json::from_slice(&header).unwrap();
    edit(&mut header);
    let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap());
    format!(
        "{header}.{}.{}",
        parts.next().unwrap(),
        parts.next().unwrap()
    )
}

#[tokio::test]
async fn a_token_signed_by_someone_else_with_the_providers_key_id_is_refused() {
    let forged = edit_header(&keys(2).sign(&claims("user_123")), |header| {
        header["kid"] = json!(keys(0).kid);
    });

    assert!(!accepts(&verifier(), &forged).await);
}

#[tokio::test]
async fn tokens_that_are_not_rs256_are_refused() {
    let body = claims("user_123");
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some(keys(0).kid.clone());
    let hmac = encode(&header, &body, &EncodingKey::from_secret(b"anything")).unwrap();
    let unsigned = format!(
        "{}.{}.",
        URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT","kid":"key-a"}"#),
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&body).unwrap()),
    );

    assert!(!accepts(&verifier(), &hmac).await);
    assert!(!accepts(&verifier(), &unsigned).await);
    assert!(!accepts(&verifier(), "not a token").await);
}

#[tokio::test]
async fn a_token_with_no_key_id_is_refused() {
    let without = edit_header(&keys(0).sign(&claims("user_123")), |header| {
        header.as_object_mut().unwrap().remove("kid");
    });

    assert!(!accepts(&verifier(), &without).await);
}

#[tokio::test]
async fn the_origin_that_asked_for_the_token_must_be_on_the_list_when_there_is_one() {
    let source = Arc::new(CountingSource::new(jwks(&[keys(0)])));
    let verifier = verifier_with(
        source,
        &["https://play.example.test", "https://*.trycloudflare.com"],
    );
    let with_origin = |origin: &str| {
        let mut body = claims("user_123");
        body["azp"] = json!(origin);
        keys(0).sign(&body)
    };

    assert!(accepts(&verifier, &with_origin("https://play.example.test")).await);
    assert!(
        accepts(
            &verifier,
            &with_origin("https://quiet-fox-42.trycloudflare.com")
        )
        .await
    );
    assert!(!accepts(&verifier, &with_origin("https://evil.example.test")).await);
    assert!(
        !accepts(
            &verifier,
            &with_origin("https://play.example.test.evil.test")
        )
        .await
    );
    assert!(
        !accepts(
            &verifier,
            &with_origin("http://quiet-fox-42.trycloudflare.com")
        )
        .await
    );
    assert!(
        !accepts(
            &verifier,
            &with_origin("https://evil.test/.trycloudflare.com")
        )
        .await
    );
    assert!(!accepts(&verifier, &with_origin("https://.trycloudflare.com")).await);
    // The star is one label, not a whole tail of them.
    assert!(
        !accepts(
            &verifier,
            &with_origin("https://a.quiet-fox-42.trycloudflare.com")
        )
        .await
    );
    // Some tokens carry no origin at all; the provider's own documentation says to skip the check.
    assert!(accepts(&verifier, &keys(0).sign(&claims("user_123"))).await);
}

#[tokio::test]
async fn any_origin_is_accepted_when_no_list_is_configured() {
    let mut body = claims("user_123");
    body["azp"] = json!("https://anywhere.example.test");

    assert!(accepts(&verifier(), &keys(0).sign(&body)).await);
}

#[tokio::test]
async fn a_new_key_published_by_the_provider_is_picked_up_after_rotation() {
    let source = Arc::new(CountingSource::new(jwks(&[keys(0)])));
    let verifier = verifier_with(source.clone(), &[]).with_refetch_cooldown(Duration::ZERO);
    assert!(accepts(&verifier, &keys(0).sign(&claims("user_123"))).await);

    source.publish(jwks(&[keys(0), keys(1)]));

    assert!(accepts(&verifier, &keys(1).sign(&claims("user_123"))).await);
    assert!(accepts(&verifier, &keys(0).sign(&claims("user_123"))).await);
}

#[tokio::test]
async fn a_stream_of_unknown_key_ids_does_not_hammer_the_provider() {
    let source = Arc::new(CountingSource::new(jwks(&[keys(0)])));
    let verifier = verifier_with(source.clone(), &[]);
    assert!(accepts(&verifier, &keys(0).sign(&claims("user_123"))).await);
    assert_eq!(source.fetches(), 1);

    for _ in 0..20 {
        assert!(!accepts(&verifier, &keys(1).sign(&claims("user_123"))).await);
    }

    assert_eq!(
        source.fetches(),
        1,
        "within the cooldown the keys are not fetched again"
    );
}

#[tokio::test]
async fn known_keys_are_served_from_the_cache() {
    let source = Arc::new(CountingSource::new(jwks(&[keys(0)])));
    let verifier = verifier_with(source.clone(), &[]);

    for _ in 0..5 {
        assert!(accepts(&verifier, &keys(0).sign(&claims("user_123"))).await);
    }

    assert_eq!(source.fetches(), 1);
}

#[tokio::test]
async fn a_key_the_provider_has_withdrawn_stops_being_trusted_once_the_copy_is_old() {
    let source = Arc::new(CountingSource::new(jwks(&[keys(0), keys(1)])));
    let verifier = verifier_with(source.clone(), &[])
        .with_refetch_cooldown(Duration::ZERO)
        .with_keys_max_age(Duration::ZERO);
    assert!(accepts(&verifier, &keys(0).sign(&claims("user_123"))).await);

    source.publish(jwks(&[keys(1)]));

    assert!(!accepts(&verifier, &keys(0).sign(&claims("user_123"))).await);
    assert!(accepts(&verifier, &keys(1).sign(&claims("user_123"))).await);
}

#[tokio::test]
async fn keys_that_are_fresh_are_not_fetched_again() {
    let source = Arc::new(CountingSource::new(jwks(&[keys(0)])));
    let verifier = verifier_with(source.clone(), &[]).with_refetch_cooldown(Duration::ZERO);

    for _ in 0..3 {
        assert!(accepts(&verifier, &keys(0).sign(&claims("user_123"))).await);
    }

    assert_eq!(source.fetches(), 1);
}

#[tokio::test]
async fn when_the_provider_goes_down_the_old_keys_keep_working() {
    let source = Arc::new(CountingSource::new(jwks(&[keys(0)])));
    let verifier = verifier_with(source.clone(), &[])
        .with_refetch_cooldown(Duration::ZERO)
        .with_keys_max_age(Duration::ZERO);
    assert!(accepts(&verifier, &keys(0).sign(&claims("user_123"))).await);

    source.set_failing(true);

    assert!(accepts(&verifier, &keys(0).sign(&claims("user_123"))).await);
    assert!(!accepts(&verifier, &keys(1).sign(&claims("user_123"))).await);
}

#[tokio::test]
async fn keys_that_have_never_loaded_are_a_server_problem_not_a_bad_token() {
    let source = Arc::new(CountingSource::new(jwks(&[keys(0)])));
    source.set_failing(true);
    let verifier = verifier_with(source.clone(), &[]);
    let token = keys(0).sign(&claims("user_123"));

    // The second try is inside the cooldown, and must still not look like a bad token.
    for _ in 0..2 {
        let result = verifier.verify(&token, Utc::now()).await;
        assert!(matches!(result, Err(ProviderError::Keys(_))), "{result:?}");
    }

    source.set_failing(false);
    let verifier = verifier.with_refetch_cooldown(Duration::ZERO);
    assert!(accepts(&verifier, &token).await);
}

#[tokio::test]
async fn a_slow_key_download_does_not_hold_up_tokens_signed_with_keys_we_have() {
    use modern_chess_social::provider::{KeyFuture, KeySource};
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Answers at once, until told to hold its answers back.
    struct Gated {
        hold: AtomicBool,
        release: tokio::sync::Notify,
    }
    impl KeySource for Gated {
        fn fetch(&self) -> KeyFuture<'_> {
            let hold = self.hold.load(Ordering::SeqCst);
            Box::pin(async move {
                if hold {
                    self.release.notified().await;
                }
                Ok(jwks(&[keys(0)]))
            })
        }
    }

    let source = Arc::new(Gated {
        hold: AtomicBool::new(false),
        release: tokio::sync::Notify::new(),
    });
    let verifier = Arc::new(
        ProviderVerifier::new(config(&[]), source.clone()).with_refetch_cooldown(Duration::ZERO),
    );
    assert!(accepts(&verifier, &keys(0).sign(&claims("user_123"))).await);

    // A token with a key id we have not seen starts a download that the provider sits on.
    source.hold.store(true, Ordering::SeqCst);
    let stuck = {
        let verifier = verifier.clone();
        tokio::spawn(async move {
            let _ = verifier
                .verify(&keys(1).sign(&claims("user_123")), Utc::now())
                .await;
        })
    };
    tokio::time::sleep(Duration::from_millis(150)).await;

    let known = tokio::time::timeout(
        Duration::from_secs(2),
        verifier.verify(&keys(0).sign(&claims("user_123")), Utc::now()),
    )
    .await;

    source.release.notify_one();
    stuck.await.unwrap();
    assert!(
        matches!(known, Ok(Ok(_))),
        "a key we already have must be served without waiting: {known:?}"
    );
}
