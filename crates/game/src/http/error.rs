use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use tracing::error;

use crate::store::StoreError;
use crate::LobbyError;

/// An error answer: a status and a short message that is safe to show a player.
/// Anything internal (SQL, engine output) is logged, never sent.
#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn not_found(message: &'static str) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn bad_request() -> Self {
        Self::new(StatusCode::BAD_REQUEST, "the request could not be read")
    }

    /// For failures that are the server's fault: logged in full, answered in one line.
    pub fn internal(cause: impl std::fmt::Display) -> Self {
        error!("request failed: {cause}");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "something went wrong")
    }
}

impl From<StoreError> for ApiError {
    fn from(err: StoreError) -> Self {
        match err {
            StoreError::NotFound => Self::not_found("match not found"),
            StoreError::Lobby(reason) => Self::from(reason),
            other => Self::internal(other),
        }
    }
}

impl From<LobbyError> for ApiError {
    fn from(reason: LobbyError) -> Self {
        let status = match reason {
            LobbyError::UnknownSeat => StatusCode::UNPROCESSABLE_ENTITY,
            LobbyError::SeatTaken | LobbyError::AlreadySeated | LobbyError::NotInLobby => {
                StatusCode::CONFLICT
            }
        };
        Self::new(status, reason.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}
