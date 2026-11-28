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
    /// Signing in with a provider is not set up on this server.
    NotConfigured,
    /// The request was understood and cannot be done as asked: `code` is for the web app to tell
    /// reasons apart, `message` is for the person.
    Rejected {
        code: &'static str,
        message: &'static str,
    },
    /// What was asked for is already someone else's.
    Taken,
    /// The caller is known and may not do this. `code` is for the web app to tell reasons apart.
    Forbidden {
        code: &'static str,
        message: &'static str,
    },
    NotFound,
    /// What was asked for cannot be done to a thing in the state it is in.
    Conflict {
        code: &'static str,
        message: &'static str,
    },
    /// The caller has done this too often lately.
    TooFast {
        code: &'static str,
        message: &'static str,
    },
    /// A part of the product this depends on is not set up or cannot be reached.
    Unavailable(&'static str),
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
            Self::NotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "sign-in is not available" })),
            )
                .into_response(),
            Self::Rejected { code, message } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({ "error": message, "code": code })),
            )
                .into_response(),
            Self::Taken => (
                StatusCode::CONFLICT,
                Json(json!({ "error": "That name is taken. Please choose another.", "code": "taken" })),
            )
                .into_response(),
            Self::Forbidden { code, message } => (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": message, "code": code })),
            )
                .into_response(),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "not found" })),
            )
                .into_response(),
            Self::Conflict { code, message } => (
                StatusCode::CONFLICT,
                Json(json!({ "error": message, "code": code })),
            )
                .into_response(),
            Self::TooFast { code, message } => (
                StatusCode::TOO_MANY_REQUESTS,
                Json(json!({ "error": message, "code": code })),
            )
                .into_response(),
            Self::Unavailable(message) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": message })),
            )
                .into_response(),
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "something went wrong" })),
            )
                .into_response(),
        }
    }
}
