use std::cell::RefCell;

use modern_chess_social_api::config::{Config, ConfigError, LogFormat};

fn config_with(values: &[(&str, &str)]) -> (Result<Config, ConfigError>, Vec<String>) {
    let asked = RefCell::new(Vec::new());
    let lookup = |key: &str| {
        asked.borrow_mut().push(key.to_string());
        values
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value.to_string())
    };
    let config = Config::from_lookup(&lookup);
    (config, asked.into_inner())
}

fn minimal() -> Vec<(&'static str, &'static str)> {
    vec![
        ("DATABASE_URL", "postgres://example/social"),
        ("SOCIAL_SIGNING_KEY_PEM", "secret-key-text"),
    ]
}

fn minimal_and(extra: &[(&'static str, &'static str)]) -> Result<Config, ConfigError> {
    // The first match wins, so the overrides go in front of the defaults.
    let mut values = extra.to_vec();
    values.extend(minimal());
    config_with(&values).0
}

#[test]
fn the_signing_key_is_required() {
    let (result, _) = config_with(&[("DATABASE_URL", "postgres://example/social")]);

    assert!(matches!(
        result,
        Err(ConfigError::Missing("SOCIAL_SIGNING_KEY_PEM"))
    ));
}

#[test]
fn a_blank_signing_key_is_refused() {
    let result = minimal_and(&[("SOCIAL_SIGNING_KEY_PEM", "   ")]);

    assert!(matches!(
        result,
        Err(ConfigError::InvalidValue("SOCIAL_SIGNING_KEY_PEM", _))
    ));
}

#[test]
fn printing_the_config_never_shows_the_signing_key() {
    let config = minimal_and(&[]).unwrap();

    let printed = format!("{config:?} {:#?}", config.auth);

    assert!(!printed.contains("secret-key-text"), "{printed}");
    assert_eq!(config.auth.signing_key_pem, "secret-key-text");
    assert_eq!(config.api.port, 3002);
}

#[test]
fn escaped_line_breaks_in_the_key_are_restored() {
    let config = minimal_and(&[("SOCIAL_SIGNING_KEY_PEM", "one\\ntwo")]).unwrap();

    assert_eq!(config.auth.signing_key_pem, "one\ntwo");
}

#[test]
fn every_setting_starts_with_the_prefix_of_its_domain() {
    let (config, asked) = config_with(&minimal());
    config.unwrap();

    let allowed = [
        "API_",
        "DATABASE_",
        "LOGGING_",
        "RUST_LOG",
        "SOCIAL_SIGNING_KEY_PEM",
    ];
    for key in &asked {
        assert!(
            allowed.iter().any(|prefix| key.starts_with(prefix)),
            "{key} does not belong to a domain"
        );
    }
}

#[test]
fn database_settings_read_the_database_prefix_and_ignore_the_old_names() {
    let config = minimal_and(&[
        ("DATABASE_MIGRATE_ON_STARTUP", "false"),
        ("DATABASE_MAX_CONNECTIONS", "3"),
    ])
    .unwrap();
    assert!(!config.database.migrate_on_startup);
    assert_eq!(config.database.max_connections, 3);

    let old_name = minimal_and(&[("DB_MIGRATE_ON_STARTUP", "false")]).unwrap();
    assert!(
        old_name.database.migrate_on_startup,
        "the old name must have no effect"
    );
}

#[test]
fn database_needs_a_url_and_a_postgres_one() {
    let (missing, _) = config_with(&[("SOCIAL_SIGNING_KEY_PEM", "k")]);
    assert!(matches!(missing, Err(ConfigError::Missing("DATABASE_URL"))));

    let wrong = config_with(&[
        ("DATABASE_URL", "other-scheme-PRIVATE-MARKER"),
        ("SOCIAL_SIGNING_KEY_PEM", "k"),
    ])
    .0;
    match wrong {
        Err(ConfigError::InvalidValue("DATABASE_URL", message)) => {
            assert!(!message.contains("PRIVATE-MARKER"), "{message}");
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn database_pool_and_timeout_must_be_positive() {
    for key in ["DATABASE_MAX_CONNECTIONS", "DATABASE_CONNECT_TIMEOUT_SECS"] {
        let result = minimal_and(&[(key, "0")]);

        assert!(
            matches!(result, Err(ConfigError::InvalidValue(name, _)) if name == key),
            "{key}"
        );
    }
}

#[test]
fn logging_settings_read_the_logging_prefix() {
    let config = minimal_and(&[
        ("LOGGING_FORMAT", "json"),
        ("LOGGING_HTTP_ACCESS_LOGS", "false"),
    ])
    .unwrap();
    assert_eq!(config.logging.format, LogFormat::Json);
    assert!(!config.logging.http_access_logs);

    let old_names = minimal_and(&[("LOG_FORMAT", "json"), ("HTTP_ACCESS_LOGS", "false")]).unwrap();
    assert_eq!(old_names.logging.format, LogFormat::Plain);
    assert!(old_names.logging.http_access_logs);

    let bad = minimal_and(&[("LOGGING_FORMAT", "xml")]);
    assert!(matches!(
        bad,
        Err(ConfigError::InvalidValue("LOGGING_FORMAT", _))
    ));
}

#[test]
fn api_address_is_checked_when_the_config_is_read() {
    let config = minimal_and(&[("API_HOST", "127.0.0.1"), ("API_PORT", "4000")]).unwrap();
    assert_eq!(config.api.addr().to_string(), "127.0.0.1:4000");

    let bad_host = minimal_and(&[("API_HOST", "not-an-address")]);
    assert!(matches!(
        bad_host,
        Err(ConfigError::InvalidValue("API_HOST", _))
    ));

    let bad_port = minimal_and(&[("API_PORT", "0")]);
    assert!(matches!(
        bad_port,
        Err(ConfigError::InvalidValue("API_PORT", _))
    ));
}
