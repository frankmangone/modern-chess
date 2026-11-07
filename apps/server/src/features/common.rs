use axum::{http::StatusCode, Json};
use serde::Serialize;
use sqlx::PgPool;

use crate::{infrastructure::db::DbError, AppState};

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub type ApiError = (StatusCode, Json<ErrorResponse>);

pub fn db_pool(state: &AppState) -> Result<&PgPool, ApiError> {
    state
        .database
        .as_ref()
        .map(|db| db.pool())
        .ok_or_else(unavailable)
}

pub fn map_db_error(err: DbError) -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: err.to_string(),
        }),
    )
}

pub fn internal_error(message: impl Into<String>) -> ApiError {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn unavailable() -> ApiError {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ErrorResponse {
            error: "database unavailable".to_string(),
        }),
    )
}

pub fn bad_request(message: impl Into<String>) -> ApiError {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn forbidden(message: impl Into<String>) -> ApiError {
    (
        StatusCode::FORBIDDEN,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn conflict(message: impl Into<String>) -> ApiError {
    (
        StatusCode::CONFLICT,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

pub fn not_found(entity: &str) -> ApiError {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: format!("{entity} not found"),
        }),
    )
}
