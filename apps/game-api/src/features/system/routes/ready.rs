use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use tracing::error;

use crate::AppState;

#[derive(Serialize)]
pub struct ReadyResponse {
    pub status: &'static str,
}

pub async fn handler(State(state): State<AppState>) -> impl IntoResponse {
    let Some(db) = state.database else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ReadyResponse { status: "degraded" }),
        );
    };

    match db.ping().await {
        Ok(_) => (StatusCode::OK, Json(ReadyResponse { status: "ready" })),
        Err(err) => {
            error!("readiness ping failed: {err}");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ReadyResponse { status: "degraded" }),
            )
        }
    }
}
