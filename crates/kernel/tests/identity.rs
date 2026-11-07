use modern_chess_kernel::{Handle, TokenClaims, UserId};

#[test]
fn user_id_serialises_as_a_plain_string() {
    let id = UserId::new();
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, format!("\"{id}\""));

    let back: UserId = serde_json::from_str(&json).unwrap();
    assert_eq!(back, id);
}

#[test]
fn user_id_parses_from_its_display_form_and_rejects_garbage() {
    let id = UserId::new();
    assert_eq!(id.to_string().parse::<UserId>().unwrap(), id);
    assert!("not-a-uuid".parse::<UserId>().is_err());
}

#[test]
fn handle_rejects_too_short_too_long_and_symbols() {
    for bad in [
        "",
        "ab",
        &"a".repeat(25),
        "has space",
        "semi;colon",
        "emoji😀x",
        "dot.dot",
    ] {
        assert!(Handle::parse(bad).is_err(), "{bad:?} should be rejected");
    }
}

#[test]
fn handle_accepts_letters_digits_underscore_dash() {
    for good in ["abc", "Guest-4821", "a_b-c", "UPPER123", &"a".repeat(24)] {
        let handle = Handle::parse(good).unwrap();
        assert_eq!(handle.as_str(), good);
    }
}

#[test]
fn token_claims_roundtrip_through_json() {
    let claims = TokenClaims {
        sub: UserId::new(),
        handle: Handle::parse("Guest-4821").unwrap(),
        exp: 1_800_000_000,
    };
    let json = serde_json::to_string(&claims).unwrap();
    let back: TokenClaims = serde_json::from_str(&json).unwrap();
    assert_eq!(back, claims);
}

#[test]
fn token_claims_reject_an_invalid_handle() {
    let json = format!(
        r#"{{"sub":"{}","handle":"no good!","exp":1}}"#,
        UserId::new()
    );
    assert!(serde_json::from_str::<TokenClaims>(&json).is_err());
}
