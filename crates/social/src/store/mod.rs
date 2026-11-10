//! Postgres repositories for the social module.

mod identity;
mod user;

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::PgPool;
use thiserror::Error;

pub use identity::AccountRepo;
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
    /// This provider identity is already linked to a user.
    #[error("that identity is already linked")]
    IdentityTaken,
    /// This user already has an identity from the same provider.
    #[error("that user already has an identity from this provider")]
    UserAlreadyLinked,
    /// The user an identity was to be linked to does not exist.
    #[error("no such user")]
    UserMissing,
    #[error("stored user is invalid: {0}")]
    Corrupt(String),
}
