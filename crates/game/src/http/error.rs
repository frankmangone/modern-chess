use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use tracing::error;

use crate::store::StoreError;

/// An error answer: a status and a short message that is safe to show a player.
/// Anything internal (SQL, engine output) is logged, never sent.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: &'static str,
}

impl ApiError {
    pub fn new(status: StatusCode, message: &'static str) -> Self {
        Self { status, message }
    }

    pub fn not_found(message: &'static str) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }
}

impl From<StoreError> for ApiError {
    fn from(err: StoreError) -> Self {
        error!("request failed: {err}");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "something went wrong")
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}
