mod health;
mod ready;

use axum::{routing::get, Router};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health::handler))
        .route("/ready", get(ready::handler))
}
