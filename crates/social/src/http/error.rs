use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use modern_chess_kernel::Unauthorized;
use serde_json::json;
use tracing::error;

/// Why a request failed. Internal causes are logged, never sent to the caller.
#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    Internal,
}

impl ApiError {
    pub fn internal(cause: impl std::fmt::Display) -> Self {
        error!("request failed: {cause}");
        Self::Internal
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Unauthorized => Unauthorized.into_response(),
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "something went wrong" })),
            )
                .into_response(),
        }
    }
}
