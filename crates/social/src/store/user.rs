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
        let result = sqlx::query("INSERT INTO users (id, handle, created_at) VALUES ($1, $2, $3)")
            .bind(user.id.as_uuid())
            .bind(user.handle.as_str())
            .bind(user.created_at)
            .execute(&self.pool)
            .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(err)) if err.constraint() == Some(HANDLE_INDEX) => {
                Err(StoreError::HandleTaken)
            }
            Err(other) => Err(other.into()),
        }
    }

    pub async fn get(&self, id: UserId) -> Result<Option<User>, StoreError> {
        let row = sqlx::query("SELECT id, handle, created_at FROM users WHERE id = $1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(user_from_row).transpose()
    }
}

fn user_from_row(row: &PgRow) -> Result<User, StoreError> {
    let handle: String = row.try_get("handle")?;
    Ok(User {
        id: UserId::from_uuid(row.try_get::<Uuid, _>("id")?),
        handle: Handle::parse(&handle).map_err(|err| StoreError::Corrupt(err.to_string()))?,
        created_at: row.try_get("created_at")?,
    })
}
