//! Postgres repositories for the social module.

mod user;

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::PgPool;
use thiserror::Error;

pub use user::UserRepo;

/// The migrations of the `social` database, embedded in the binary.
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Brings the `social` database up to date. Safe to run on every start.
pub async fn migrate(pool: &PgPool) -> Result<(), MigrateError> {
    MIGRATOR.run(pool).await
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    /// Another user already has this handle (compared without regard to case).
    #[error("that handle is already taken")]
    HandleTaken,
    #[error("stored user is invalid: {0}")]
    Corrupt(String),
}
