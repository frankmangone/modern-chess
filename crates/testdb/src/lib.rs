//! Throwaway databases for tests. Dev-only: nothing ships this crate.
//!
//! Every call creates a fresh, empty database on the development Postgres (`make db-up`),
//! runs the test against it, and drops it afterwards, whether the test passed or not.
//! The administrative connection comes from `TEST_DATABASE_URL`; without it the
//! development container's address is used.

use std::future::Future;
use std::panic::resume_unwind;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

const DEFAULT_ADMIN_URL: &str = "postgres://modern_chess:modern_chess@localhost:5455/postgres";

fn admin_options() -> PgConnectOptions {
    let url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| DEFAULT_ADMIN_URL.to_string());
    url.parse()
        .expect("TEST_DATABASE_URL should be a postgres url")
}

async fn admin_connection() -> PgConnection {
    PgConnection::connect_with(&admin_options())
        .await
        .unwrap_or_else(|err| {
            panic!("cannot reach the test Postgres ({err}); start it with `make db-up`")
        })
}

/// Runs `test` against a new empty database and drops the database afterwards.
pub async fn with_db<F, Fut>(test: F)
where
    F: FnOnce(PgPool) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    let name = format!("mc_test_{}", Uuid::new_v4().simple());

    let mut admin = admin_connection().await;
    admin
        .execute(format!("CREATE DATABASE {name}").as_str())
        .await
        .expect("creating the throwaway database");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect_with(admin_options().database(&name))
        .await
        .expect("connecting to the throwaway database");

    // Run on its own task so a panicking test still reaches the cleanup below.
    let outcome = tokio::spawn(test(pool.clone())).await;

    pool.close().await;
    admin
        .execute(format!("DROP DATABASE IF EXISTS {name} WITH (FORCE)").as_str())
        .await
        .expect("dropping the throwaway database");

    if let Err(failure) = outcome {
        if failure.is_panic() {
            resume_unwind(failure.into_panic());
        }
        panic!("the test task was cancelled");
    }
}
