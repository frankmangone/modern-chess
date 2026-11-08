use chrono::{DateTime, Duration, TimeZone, Utc};
use modern_chess_kernel::{TokenError, TokenVerifier, UserId};
use modern_chess_testkeys::{claims, sign_with_hmac, TestKeys};
use serde_json::json;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap()
}

fn verifier(keys: &TestKeys) -> TokenVerifier {
    TokenVerifier::from_public_pem(&keys.public_pem).unwrap()
}

#[test]
fn a_valid_token_verifies_and_yields_its_claims() {
    let keys = TestKeys::generate();
    let user = UserId::new();
    let exp = (now() + Duration::days(7)).timestamp();

    let verified = verifier(&keys)
        .verify(&keys.token_for(user, "Guest-1234", exp), now())
        .unwrap();

    assert_eq!(verified, claims(user, "Guest-1234", exp));
}

#[test]
fn a_token_signed_with_a_different_key_is_rejected() {
    let signer = TestKeys::generate();
    let other = TestKeys::generate();
    let token = signer.token_for(
        UserId::new(),
        "Guest-1234",
        (now() + Duration::days(1)).timestamp(),
    );

    assert_eq!(
        verifier(&other).verify(&token, now()),
        Err(TokenError::Invalid)
    );
}

#[test]
fn an_expired_token_is_rejected() {
    let keys = TestKeys::generate();
    let token = keys.token_for(
        UserId::new(),
        "Guest-1234",
        (now() - Duration::seconds(10)).timestamp(),
    );

    assert_eq!(
        verifier(&keys).verify(&token, now()),
        Err(TokenError::Expired)
    );
}

#[test]
fn a_token_that_expires_this_very_second_is_rejected() {
    let keys = TestKeys::generate();
    let token = keys.token_for(UserId::new(), "Guest-1234", now().timestamp());

    assert_eq!(
        verifier(&keys).verify(&token, now()),
        Err(TokenError::Expired)
    );
}

#[test]
fn a_tampered_token_is_rejected() {
    let keys = TestKeys::generate();
    let exp = (now() + Duration::days(1)).timestamp();
    let mine = keys.token_for(UserId::new(), "Guest-1234", exp);
    let theirs = keys.token_for(UserId::new(), "Admin-0001", exp);

    // Someone else's claims under my signature.
    let forged = format!(
        "{}.{}.{}",
        mine.split('.').next().unwrap(),
        theirs.split('.').nth(1).unwrap(),
        mine.split('.').nth(2).unwrap()
    );

    assert_eq!(
        verifier(&keys).verify(&forged, now()),
        Err(TokenError::Invalid)
    );
}

#[test]
fn garbage_and_empty_tokens_are_rejected() {
    let keys = TestKeys::generate();
    let verifier = verifier(&keys);

    for garbage in ["", "abc", "a.b.c", "....", "Bearer x", "ey.ey.ey", "\u{0}"] {
        assert_eq!(
            verifier.verify(garbage, now()),
            Err(TokenError::Invalid),
            "{garbage:?}"
        );
    }
}

#[test]
fn a_token_forged_with_hmac_using_the_public_key_as_secret_is_rejected() {
    let keys = TestKeys::generate();
    let forged = sign_with_hmac(
        keys.public_pem.as_bytes(),
        &claims(
            UserId::new(),
            "Guest-1234",
            (now() + Duration::days(1)).timestamp(),
        ),
    );

    assert_eq!(
        verifier(&keys).verify(&forged, now()),
        Err(TokenError::Invalid)
    );
}

#[test]
fn a_token_without_an_expiry_is_rejected() {
    let keys = TestKeys::generate();
    let token = keys.sign(&json!({ "sub": UserId::new(), "handle": "Guest-1234" }));

    assert_eq!(
        verifier(&keys).verify(&token, now()),
        Err(TokenError::Invalid)
    );
}

#[test]
fn a_public_key_that_is_not_a_key_is_reported() {
    assert!(matches!(
        TokenVerifier::from_public_pem("not a key"),
        Err(TokenError::InvalidKey)
    ));
}
