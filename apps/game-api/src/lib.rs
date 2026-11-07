pub mod config;
pub mod features;
pub mod infrastructure;

use axum::{http::HeaderName, Router};
use config::Config;
use infrastructure::db::Database;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::net::TcpListener;
use tokio::time::{timeout, Duration};
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use tracing::{error, info, warn};

const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

#[derive(Clone)]
pub struct AppState {
    pub database: Option<Database>,
    pub config: Config,
}

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    load_env();
    let config = Config::from_env()?;
    infrastructure::logging::init(&config.logging)?;

    info!("starting modern-chess-game-api");

    let db_pool = init_pool(&config).await?;
    if config.db.migrate_on_startup {
        info!("running database migrations");
        let schema = infrastructure::db::migrations::run(&db_pool).await?;
        info!(
            "schema ready latest_applied={:?} latest_known={:?} applied_count={} pending_count={}",
            schema.latest_applied_version,
            schema.latest_known_version,
            schema.applied_count,
            schema.pending_versions.len()
        );
    } else {
        warn!("database migrations on startup are disabled");
        let schema = infrastructure::db::migrations::status(&db_pool).await?;
        if !schema.is_current() {
            let message = format!(
                "database schema is outdated; pending migration versions: {:?}",
                schema.pending_versions
            );
            error!("{message}");
            return Err(std::io::Error::other(message).into());
        }
    }

    let database = Some(Database::new(db_pool.clone()));
    let state = AppState {
        database,
        config: config.clone(),
    };

    let game = modern_chess_game::GameState::new(db_pool.clone());
    if config.db.seed_defaults_on_startup {
        let report =
            modern_chess_game::seed::run(&game.specs, &game.pages, chrono::Utc::now()).await?;
        info!(
            "seeded {} specs and {} pages ({} skipped)",
            report.specs,
            report.pages,
            report.skipped.len()
        );
    }

    let app = app_router(state.clone(), game);
    let listener = TcpListener::bind(config.api_addr()?).await?;
    info!("http api listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;
    Ok(())
}

fn load_env() {
    let _ = dotenvy::from_filename("apps/game-api/.env");
    let _ = dotenvy::dotenv();
}

pub fn app_router(state: AppState, game: modern_chess_game::GameState) -> Router {
    let request_id_layer = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(
            X_REQUEST_ID.clone(),
            MakeRequestUuid,
        ))
        .layer(PropagateRequestIdLayer::new(X_REQUEST_ID.clone()));

    let base = Router::new()
        .merge(features::system::router())
        .with_state(state.clone())
        .nest("/api/game", modern_chess_game::router(game))
        .layer(request_id_layer);

    if state.config.logging.http_access_logs {
        base.layer(TraceLayer::new_for_http())
    } else {
        base
    }
}

async fn init_pool(config: &Config) -> Result<PgPool, sqlx::Error> {
    info!(
        "connecting to postgres (timeout={}s max_connections={})",
        config.db.connect_timeout_secs, config.db.max_connections
    );

    let connection = timeout(
        Duration::from_secs(config.db.connect_timeout_secs),
        PgPoolOptions::new()
            .max_connections(config.db.max_connections)
            .connect(&config.db.url),
    )
    .await;

    let pool = match connection {
        Ok(Ok(pool)) => pool,
        Ok(Err(err)) => {
            error!("postgres connection failed: {err}");
            return Err(err);
        }
        Err(_) => {
            error!(
                "postgres connection timed out after {}s",
                config.db.connect_timeout_secs
            );
            return Err(sqlx::Error::PoolTimedOut);
        }
    };

    info!("connected to postgres");
    Ok(pool)
}
