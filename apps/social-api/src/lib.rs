//! The social process: guest sign-up, signing in with an outside provider, and who-am-I,
//! backed by its own database. It signs the tokens the game process verifies, and never talks
//! to the game process.

pub mod config;

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use config::{Config, LogFormat};
use modern_chess_social::provider::ProviderVerifier;
use modern_chess_social::{SocialState, TokenIssuer};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tokio::time::{timeout, Duration};
use tower_http::{catch_panic::CatchPanicLayer, trace::TraceLayer};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub async fn run() -> Result<(), BoxError> {
    load_env();
    let config = Config::from_env()?;
    init_logging(&config)?;

    info!("starting modern-chess-social-api");
    let pool = connect(&config).await?;
    if config.database.migrate_on_startup {
        info!("running database migrations");
        modern_chess_social::migrate(&pool).await?;
    }

    let issuer = TokenIssuer::from_private_pem(&config.auth.signing_key_pem).map_err(|_| {
        std::io::Error::other("SOCIAL_SIGNING_KEY_PEM is not a valid Ed25519 private key")
    })?;
    let mut state = SocialState::new(pool.clone(), Arc::new(issuer));
    match config.auth.provider.clone() {
        Some(provider) => {
            info!("sign-in provider: {}", provider.name);
            let verifier = ProviderVerifier::connect(provider)
                .map_err(|err| std::io::Error::other(err.to_string()))?;
            state = state.with_provider(Arc::new(verifier));
        }
        None => info!("no sign-in provider configured: guests only"),
    }

    let app = app_router(pool, state, config.logging.http_access_logs);
    let listener = TcpListener::bind(config.api.addr()).await?;
    info!("http api listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

/// Health and readiness at the root, the social routes under `/api/social`.
pub fn app_router(pool: PgPool, state: SocialState, access_logs: bool) -> Router {
    let app = guard_panics(
        Router::new()
            .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
            .route("/ready", get(ready))
            .with_state(pool)
            .nest("/api/social", modern_chess_social::router(state)),
    );

    if access_logs {
        app.layer(TraceLayer::new_for_http())
    } else {
        app
    }
}

/// Answers a panic in any handler with a plain 500 and keeps the server serving. The panic
/// message is logged, never sent.
pub fn guard_panics(router: Router) -> Router {
    router.layer(CatchPanicLayer::custom(panic_response))
}

fn panic_response(cause: Box<dyn std::any::Any + Send>) -> axum::response::Response {
    let cause = cause
        .downcast_ref::<&str>()
        .map(|message| message.to_string())
        .or_else(|| cause.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a non-text panic".to_string());
    error!("a request handler panicked: {cause}");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": "something went wrong" })),
    )
        .into_response()
}

async fn ready(State(pool): State<PgPool>) -> (StatusCode, Json<Value>) {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
    {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "ready" }))),
        Err(err) => {
            error!("readiness ping failed: {err}");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "degraded" })),
            )
        }
    }
}

fn load_env() {
    let _ = dotenvy::from_filename("apps/social-api/.env");
    let _ = dotenvy::dotenv();
}

fn init_logging(config: &Config) -> Result<(), BoxError> {
    let filter = EnvFilter::try_new(&config.logging.level)
        .unwrap_or_else(|_| EnvFilter::new("modern_chess_social_api=info,info"));
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true);
    if config.logging.format == LogFormat::Json {
        builder.json().try_init()?;
    } else {
        builder.compact().try_init()?;
    }
    Ok(())
}

async fn connect(config: &Config) -> Result<PgPool, BoxError> {
    info!(
        "connecting to postgres (timeout={}s max_connections={})",
        config.database.connect_timeout_secs, config.database.max_connections
    );
    let pool = timeout(
        Duration::from_secs(config.database.connect_timeout_secs),
        PgPoolOptions::new()
            .max_connections(config.database.max_connections)
            .connect(&config.database.url),
    )
    .await
    .map_err(|_| std::io::Error::other("postgres connection timed out"))??;
    info!("connected to postgres");
    Ok(pool)
}
