use modern_chess_social_api::config::{Config, ConfigError};

fn lookup(values: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
    move |key| {
        values
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value.to_string())
    }
}

#[test]
fn the_signing_key_is_required() {
    let result = Config::from_lookup(&lookup(&[("DATABASE_URL", "postgres://example/social")]));

    assert!(matches!(
        result,
        Err(ConfigError::Missing("SOCIAL_SIGNING_KEY_PEM"))
    ));
}

#[test]
fn printing_the_config_never_shows_the_signing_key() {
    let config = Config::from_lookup(&lookup(&[
        ("DATABASE_URL", "postgres://example/social"),
        ("SOCIAL_SIGNING_KEY_PEM", "secret-key-text"),
    ]))
    .unwrap();

    let printed = format!("{config:?} {:#?}", config.auth);

    assert!(!printed.contains("secret-key-text"), "{printed}");
    assert_eq!(config.auth.signing_key_pem, "secret-key-text");
    assert_eq!(config.api.port, 3002);
}

#[test]
fn escaped_line_breaks_in_the_key_are_restored() {
    let config = Config::from_lookup(&lookup(&[
        ("DATABASE_URL", "postgres://example/social"),
        ("SOCIAL_SIGNING_KEY_PEM", "one\\ntwo"),
    ]))
    .unwrap();

    assert_eq!(config.auth.signing_key_pem, "one\ntwo");
}
