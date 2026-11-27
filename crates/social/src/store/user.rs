use modern_chess_kernel::{Handle, UserId};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::StoreError;
use crate::User;

const HANDLE_INDEX: &str = "uq_users_handle_lower";

#[derive(Clone)]
pub struct UserRepo {
    pool: PgPool,
}

impl UserRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Stores a user. A handle that is already in use, in any letter case, is reported as
    /// `HandleTaken` so the caller can choose another.
    pub async fn insert(&self, user: &User) -> Result<(), StoreError> {
        insert_on(&self.pool, user).await
    }

    /// A live user. A deleted account is no user at all.
    pub async fn get(&self, id: UserId) -> Result<Option<User>, StoreError> {
        let row = sqlx::query(
            "SELECT id, handle, created_at, setup_pending FROM users \
             WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(user_from_row).transpose()
    }

    /// Whether nobody has this handle, in any letter case. A deleted account's handle is already
    /// a `deleted-...` name, so it never blocks one.
    pub async fn handle_is_free(&self, handle: &Handle) -> Result<bool, StoreError> {
        let taken: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM users WHERE lower(handle) = lower($1))",
        )
        .bind(handle.as_str())
        .fetch_one(&self.pool)
        .await?;
        Ok(!taken)
    }

    /// Gives a live user a new handle. `HandleTaken` when another user has it (in any letter
    /// case; changing only the case of your own is fine), `UserMissing` when there is no such
    /// live user.
    pub async fn set_handle(&self, id: UserId, handle: &Handle) -> Result<(), StoreError> {
        let result =
            sqlx::query("UPDATE users SET handle = $2 WHERE id = $1 AND deleted_at IS NULL")
                .bind(id.as_uuid())
                .bind(handle.as_str())
                .execute(&self.pool)
                .await;
        match result {
            Ok(done) if done.rows_affected() == 0 => Err(StoreError::UserMissing),
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(err)) if err.constraint() == Some(HANDLE_INDEX) => {
                Err(StoreError::HandleTaken)
            }
            Err(other) => Err(other.into()),
        }
    }

    /// Ends a user's set-up. False when there is no such live user.
    pub async fn finish_setup(&self, id: UserId) -> Result<bool, StoreError> {
        let done = sqlx::query(
            "UPDATE users SET setup_pending = FALSE WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await?;
        Ok(done.rows_affected() > 0)
    }
}

/// Inserts on a pool or inside a transaction.
pub(super) async fn insert_on<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    user: &User,
) -> Result<(), StoreError> {
    let result = sqlx::query(
        "INSERT INTO users (id, handle, created_at, setup_pending) VALUES ($1, $2, $3, $4)",
    )
    .bind(user.id.as_uuid())
    .bind(user.handle.as_str())
    .bind(user.created_at)
    .bind(user.setup_pending)
    .execute(executor)
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(err)) if err.constraint() == Some(HANDLE_INDEX) => {
            Err(StoreError::HandleTaken)
        }
        Err(other) => Err(other.into()),
    }
}

fn user_from_row(row: &PgRow) -> Result<User, StoreError> {
    let handle: String = row.try_get("handle")?;
    Ok(User {
        id: UserId::from_uuid(row.try_get::<Uuid, _>("id")?),
        handle: Handle::parse(&handle).map_err(|err| StoreError::Corrupt(err.to_string()))?,
        created_at: row.try_get("created_at")?,
        setup_pending: row.try_get("setup_pending")?,
    })
}
