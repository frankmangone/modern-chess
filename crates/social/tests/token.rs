use chrono::{DateTime, Duration, TimeZone, Utc};
use modern_chess_kernel::{Handle, TokenError, TokenVerifier};
use modern_chess_social::{TokenIssuer, User};
use modern_chess_testkeys::TestKeys;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap()
}

fn guest() -> User {
    User::new_guest(Handle::parse("Guest-1234").unwrap(), now())
}

#[test]
fn guest_token_verifies_with_the_public_key() {
    let keys = TestKeys::generate();
    let issuer = TokenIssuer::from_private_pem(&keys.private_pem).unwrap();
    let user = guest();

    let token = issuer.issue(&user, now()).unwrap();
    let claims = TokenVerifier::from_public_pem(&keys.public_pem)
        .unwrap()
        .verify(&token, now())
        .unwrap();

    assert_eq!(claims.sub, user.id);
    assert_eq!(claims.handle, user.handle);
}

#[test]
fn guest_token_lasts_seven_days() {
    let keys = TestKeys::generate();
    let issuer = TokenIssuer::from_private_pem(&keys.private_pem).unwrap();
    let verifier = TokenVerifier::from_public_pem(&keys.public_pem).unwrap();
    let token = issuer.issue(&guest(), now()).unwrap();

    let claims = verifier.verify(&token, now()).unwrap();
    assert_eq!(claims.exp, (now() + Duration::days(7)).timestamp());

    let almost = now() + Duration::days(7) - Duration::seconds(1);
    assert!(verifier.verify(&token, almost).is_ok());
    let over = now() + Duration::days(7);
    assert_eq!(verifier.verify(&token, over), Err(TokenError::Expired));
}

#[test]
fn the_issuer_publishes_the_public_half_of_its_key() {
    let keys = TestKeys::generate();
    let issuer = TokenIssuer::from_private_pem(&keys.private_pem).unwrap();
    let token = issuer.issue(&guest(), now()).unwrap();

    let verifier = TokenVerifier::from_public_pem(issuer.public_pem()).unwrap();

    assert!(verifier.verify(&token, now()).is_ok());
}

#[test]
fn something_that_is_not_a_private_key_is_reported() {
    let real = TestKeys::generate().private_pem;
    let truncated = real[..real.len() / 2].to_string();

    for bad in ["", "not a key", truncated.as_str()] {
        assert!(
            matches!(
                TokenIssuer::from_private_pem(bad),
                Err(TokenError::InvalidKey)
            ),
            "{bad:?}"
        );
    }
}

#[test]
fn a_public_key_is_not_accepted_as_a_signing_key() {
    let keys = TestKeys::generate();
    assert!(TokenIssuer::from_private_pem(&keys.public_pem).is_err());
}
