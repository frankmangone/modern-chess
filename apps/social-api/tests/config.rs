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
        "SOCIAL_PROVIDER_",
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

const CLERK: [(&str, &str); 2] = [
    ("SOCIAL_PROVIDER_NAME", "clerk"),
    (
        "SOCIAL_PROVIDER_ISSUER",
        "https://quiet-fox-12.clerk.accounts.dev",
    ),
];

#[test]
fn without_an_issuer_there_is_no_provider() {
    let config = minimal_and(&[]).unwrap();

    assert!(config.auth.provider.is_none());
}

#[test]
fn a_provider_needs_a_name_with_its_issuer() {
    let result = minimal_and(&[CLERK[1]]);

    assert!(matches!(
        result,
        Err(ConfigError::Missing("SOCIAL_PROVIDER_NAME"))
    ));
}

#[test]
fn the_keys_address_defaults_to_the_standard_place_under_the_issuer() {
    let config = minimal_and(&CLERK).unwrap();
    let provider = config.auth.provider.unwrap();

    assert_eq!(provider.name, "clerk");
    assert_eq!(provider.issuer, "https://quiet-fox-12.clerk.accounts.dev");
    assert_eq!(
        provider.jwks_url,
        "https://quiet-fox-12.clerk.accounts.dev/.well-known/jwks.json"
    );
    assert!(provider.authorized_parties.is_empty());

    let slash = minimal_and(&[
        CLERK[0],
        ("SOCIAL_PROVIDER_ISSUER", "https://idp.example.test/"),
    ])
    .unwrap();
    let slash = slash.auth.provider.unwrap();
    assert_eq!(
        slash.jwks_url,
        "https://idp.example.test/.well-known/jwks.json"
    );
    assert_eq!(
        slash.issuer, "https://idp.example.test/",
        "tokens carry the issuer exactly as the provider writes it"
    );
}

#[test]
fn the_keys_address_can_be_set_and_the_authorized_origins_listed() {
    let config = minimal_and(&[
        CLERK[0],
        CLERK[1],
        ("SOCIAL_PROVIDER_JWKS_URL", "https://keys.example.test/jwks"),
        (
            "SOCIAL_PROVIDER_AUTHORIZED_PARTIES",
            " http://localhost:5173 , https://*.trycloudflare.com,, ",
        ),
    ])
    .unwrap();
    let provider = config.auth.provider.unwrap();

    assert_eq!(provider.jwks_url, "https://keys.example.test/jwks");
    assert_eq!(
        provider.authorized_parties,
        ["http://localhost:5173", "https://*.trycloudflare.com"]
    );
}

#[test]
fn the_provider_must_be_reached_over_https_except_on_this_machine() {
    for bad in [
        "http://idp.example.test",
        "ftp://idp.example.test",
        "idp.example.test",
    ] {
        let result = minimal_and(&[CLERK[0], ("SOCIAL_PROVIDER_ISSUER", bad)]);
        assert!(
            matches!(
                result,
                Err(ConfigError::InvalidValue("SOCIAL_PROVIDER_ISSUER", _))
            ),
            "{bad}"
        );
    }
    let keys = minimal_and(&[
        CLERK[0],
        CLERK[1],
        ("SOCIAL_PROVIDER_JWKS_URL", "http://keys.example.test"),
    ]);
    assert!(matches!(
        keys,
        Err(ConfigError::InvalidValue("SOCIAL_PROVIDER_JWKS_URL", _))
    ));

    for local in ["http://localhost:9000", "http://127.0.0.1:9000"] {
        assert!(
            minimal_and(&[CLERK[0], ("SOCIAL_PROVIDER_ISSUER", local)]).is_ok(),
            "{local}"
        );
    }
    // Looks local, is not: the host is `localhost.evil.test`.
    let sneaky = minimal_and(&[
        CLERK[0],
        ("SOCIAL_PROVIDER_ISSUER", "http://localhost.evil.test"),
    ]);
    assert!(sneaky.is_err());
}

#[test]
fn the_provider_name_is_a_short_plain_label() {
    for bad in [
        "",
        "Clerk",
        "with space",
        "a/b",
        "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
    ] {
        let result = minimal_and(&[("SOCIAL_PROVIDER_NAME", bad), CLERK[1]]);
        assert!(
            matches!(
                result,
                Err(ConfigError::InvalidValue("SOCIAL_PROVIDER_NAME", _))
            ),
            "{bad:?}"
        );
    }
}
