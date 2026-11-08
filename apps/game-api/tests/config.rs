use std::cell::RefCell;

use modern_chess_game_api::config::{Config, ConfigError};

const SENTINEL: &str = "this-must-never-be-read";

fn config_with(values: &[(&str, &str)]) -> (Result<Config, ConfigError>, Vec<String>) {
    let asked = RefCell::new(Vec::new());
    let lookup = |key: &str| {
        asked.borrow_mut().push(key.to_string());
        if key == "SOCIAL_SIGNING_KEY_PEM" {
            return Some(SENTINEL.to_string());
        }
        values
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value.to_string())
    };
    let config = Config::from_lookup(&lookup);
    (config, asked.into_inner())
}

#[test]
fn game_api_config_has_no_private_key() {
    let (config, asked) = config_with(&[
        ("DATABASE_URL", "postgres://example/game"),
        ("AUTH_PUBLIC_KEY_PEM", "public-key-text"),
    ]);
    let config = config.unwrap();

    assert!(
        !asked.iter().any(|key| key.contains("SIGNING")),
        "game-api asked for {asked:?}: it must never read the private signing key"
    );
    assert!(asked.contains(&"AUTH_PUBLIC_KEY_PEM".to_string()));
    assert!(!format!("{config:?}").contains(SENTINEL));
    assert_eq!(config.auth.public_key_pem, "public-key-text");
}

#[test]
fn game_api_refuses_to_start_without_a_public_key() {
    let (config, _) = config_with(&[("DATABASE_URL", "postgres://example/game")]);

    assert!(matches!(
        config,
        Err(ConfigError::Missing("AUTH_PUBLIC_KEY_PEM"))
    ));
}

#[test]
fn escaped_line_breaks_in_the_key_are_restored() {
    let (config, _) = config_with(&[
        ("DATABASE_URL", "postgres://example/game"),
        ("AUTH_PUBLIC_KEY_PEM", "line one\\nline two"),
    ]);

    assert_eq!(config.unwrap().auth.public_key_pem, "line one\nline two");
}
