pub mod repository;
pub mod routes;

use axum::Router;

use crate::AppState;

pub fn router() -> Router<AppState> {
    routes::router()
}
