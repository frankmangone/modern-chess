//! Postgres repositories for the game module. They take and return domain structs, never
//! rows, and own every SQL statement the module runs.

mod command;
mod error;
mod instance;
mod page;
mod spec;

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::PgPool;

pub use command::CommandRepo;
pub use error::StoreError;
pub use instance::InstanceRepo;
pub use page::PageRepo;
pub use spec::SpecRepo;

/// The migrations of the `game` database, embedded in the binary.
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Brings the `game` database up to date. Safe to run on every start.
pub async fn migrate(pool: &PgPool) -> Result<(), MigrateError> {
    MIGRATOR.run(pool).await
}
