use std::cell::RefCell;

use modern_chess_game_api::config::{Config, ConfigError, ImageStorageConfig};

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

fn minimal() -> Vec<(&'static str, &'static str)> {
    vec![
        ("DATABASE_URL", "postgres://example/game"),
        ("AUTH_PUBLIC_KEY_PEM", "public-key-text"),
    ]
}

fn minimal_and(extra: &[(&'static str, &'static str)]) -> Result<Config, ConfigError> {
    let mut values = minimal();
    values.extend_from_slice(extra);
    config_with(&values).0
}

#[test]
fn every_setting_starts_with_the_prefix_of_its_domain() {
    let (config, asked) = config_with(&minimal());
    config.unwrap();

    let prefixes = ["API_", "DATABASE_", "AUTH_", "LOGGING_", "IMAGES_"];
    for key in &asked {
        // RUST_LOG is the logging library's own name, so it keeps it.
        assert!(
            key == "RUST_LOG" || prefixes.iter().any(|prefix| key.starts_with(prefix)),
            "{key} has no domain prefix"
        );
    }
}

#[test]
fn database_settings_read_the_database_prefix_and_ignore_the_old_names() {
    let config = minimal_and(&[
        ("DATABASE_MIGRATE_ON_STARTUP", "false"),
        ("DATABASE_SEED_DEFAULT_SPECS", "false"),
    ])
    .unwrap();
    assert!(!config.database.migrate_on_startup);
    assert!(!config.database.seed_defaults_on_startup);

    let old = minimal_and(&[("DB_MIGRATE_ON_STARTUP", "false")]).unwrap();
    assert!(
        old.database.migrate_on_startup,
        "the old name is not read any more"
    );
}

#[test]
fn database_needs_a_url_and_a_postgres_one() {
    let (config, _) = config_with(&[("AUTH_PUBLIC_KEY_PEM", "key")]);
    assert!(matches!(config, Err(ConfigError::Missing("DATABASE_URL"))));

    // The value is not echoed: a real url carries a password.
    let result = config_with(&[
        ("DATABASE_URL", "mysql://example/game-secret-marker"),
        ("AUTH_PUBLIC_KEY_PEM", "key"),
    ])
    .0;
    let Err(error) = result else {
        panic!("a mysql url must be refused")
    };
    assert!(matches!(
        error,
        ConfigError::InvalidValue("DATABASE_URL", _)
    ));
    assert!(!error.to_string().contains("secret-marker"));
}

#[test]
fn database_pool_and_timeout_must_be_positive() {
    for key in ["DATABASE_MAX_CONNECTIONS", "DATABASE_CONNECT_TIMEOUT_SECS"] {
        let result = minimal_and(&[(key, "0")]);
        assert!(
            matches!(&result, Err(ConfigError::InvalidValue(name, _)) if *name == key),
            "{key}=0 must be refused"
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
    assert_eq!(format!("{:?}", config.logging.format), "Json");
    assert!(!config.logging.http_access_logs);

    let result = minimal_and(&[("LOGGING_FORMAT", "xml")]);
    assert!(matches!(
        result,
        Err(ConfigError::InvalidValue("LOGGING_FORMAT", _))
    ));
}

#[test]
fn api_address_is_checked_when_the_config_is_read() {
    let config = minimal_and(&[("API_HOST", "127.0.0.1"), ("API_PORT", "4000")]).unwrap();
    assert_eq!(config.api.addr().to_string(), "127.0.0.1:4000");

    let bad_host = minimal_and(&[("API_HOST", "not a host")]);
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

#[test]
fn a_blank_public_key_is_refused() {
    let result = config_with(&[
        ("DATABASE_URL", "postgres://example/game"),
        ("AUTH_PUBLIC_KEY_PEM", "   "),
    ])
    .0;

    assert!(matches!(
        result,
        Err(ConfigError::InvalidValue("AUTH_PUBLIC_KEY_PEM", _))
    ));
}

#[test]
fn images_are_served_from_the_local_store_unless_told_otherwise() {
    let config = minimal_and(&[]).unwrap();
    assert_eq!(
        config.images.public_url,
        "http://127.0.0.1:9000/modern-chess"
    );

    let set = minimal_and(&[("IMAGES_PUBLIC_URL", "https://img.example.com/covers/")]).unwrap();
    assert_eq!(set.images.public_url, "https://img.example.com/covers");
}

#[test]
fn the_image_address_must_be_a_plain_http_address() {
    for bad in [
        "",
        "localhost:9000",
        "ftp://img.example.com",
        "http://",
        "https://a.test/x?token=1",
        "https://a.test/x#top",
    ] {
        let result = minimal_and(&[("IMAGES_PUBLIC_URL", bad)]);
        assert!(
            matches!(
                result,
                Err(ConfigError::InvalidValue("IMAGES_PUBLIC_URL", _))
            ),
            "{bad:?} must be refused"
        );
    }
}

fn storage_with(values: &[(&str, &str)]) -> (Result<ImageStorageConfig, ConfigError>, Vec<String>) {
    let asked = RefCell::new(Vec::new());
    let lookup = |key: &str| {
        asked.borrow_mut().push(key.to_string());
        values
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| value.to_string())
    };
    let config = ImageStorageConfig::from_lookup(&lookup);
    (config, asked.into_inner())
}

const KEYS: [(&str, &str); 2] = [
    ("IMAGES_ACCESS_KEY", "an-access-key"),
    ("IMAGES_SECRET_KEY", "a-secret-key"),
];

#[test]
fn the_uploader_defaults_to_the_local_store_and_needs_credentials() {
    let (config, asked) = storage_with(&KEYS);
    let config = config.unwrap();

    assert_eq!(config.endpoint, "http://127.0.0.1:9000");
    assert_eq!(config.region, "us-east-1");
    assert_eq!(config.bucket, "modern-chess");
    assert!(config.force_path_style);
    assert!(
        asked.iter().all(|key| key.starts_with("IMAGES_")),
        "{asked:?}"
    );

    let (missing, _) = storage_with(&[("IMAGES_ACCESS_KEY", "k")]);
    assert!(matches!(
        missing,
        Err(ConfigError::Missing("IMAGES_SECRET_KEY"))
    ));
    let (missing, _) = storage_with(&[("IMAGES_SECRET_KEY", "s")]);
    assert!(matches!(
        missing,
        Err(ConfigError::Missing("IMAGES_ACCESS_KEY"))
    ));
}

#[test]
fn the_uploaders_settings_are_checked() {
    let cases: [(&str, &str); 6] = [
        ("IMAGES_ENDPOINT", "localhost:9000"),
        ("IMAGES_ENDPOINT", "ftp://store.test"),
        ("IMAGES_BUCKET", "Has Capitals"),
        ("IMAGES_BUCKET", "ab"),
        ("IMAGES_REGION", "  "),
        ("IMAGES_ACCESS_KEY", " "),
    ];
    for (key, bad) in cases {
        let mut values = KEYS.to_vec();
        values.retain(|(name, _)| *name != key);
        values.push((key, bad));
        let (result, _) = storage_with(&values);
        assert!(
            matches!(&result, Err(ConfigError::InvalidValue(name, _)) if *name == key),
            "{key}={bad:?} must be refused"
        );
    }
}

#[test]
fn the_uploaders_secret_never_appears_when_the_settings_are_printed() {
    let (config, _) = storage_with(&KEYS);

    assert!(!format!("{:?}", config.unwrap()).contains("a-secret-key"));
}

#[test]
fn the_web_server_never_asks_for_storage_credentials() {
    let (_, asked) = config_with(&minimal());

    assert!(
        !asked
            .iter()
            .any(|key| key == "IMAGES_ACCESS_KEY" || key == "IMAGES_SECRET_KEY"),
        "the server only builds addresses; it must not hold the means to write images: {asked:?}"
    );
}
