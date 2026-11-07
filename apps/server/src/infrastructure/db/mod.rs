pub mod migrations;
pub mod models;

use serde_json::Value;
use sqlx::PgPool;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn ping(&self) -> Result<(), DbError> {
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum DbError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("invalid persisted value for {field}: {value}")]
    InvalidData { field: &'static str, value: String },
    #[error("invalid game spec: {0}")]
    InvalidSpec(String),
}

pub fn value_as_str(field: &'static str, value: &Value) -> Result<String, DbError> {
    match value.as_str() {
        Some(v) => Ok(v.to_string()),
        None => Err(DbError::InvalidData {
            field,
            value: value.to_string(),
        }),
    }
}
