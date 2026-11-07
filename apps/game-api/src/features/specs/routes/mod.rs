mod list_specs;
mod seed_defaults;

use axum::{
    routing::{get, post},
    Router,
};

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_specs::handler))
        .route("/seed-defaults", post(seed_defaults::handler))
}
