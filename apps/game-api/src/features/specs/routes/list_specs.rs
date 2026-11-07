use axum::{extract::State, Json};
use serde::Serialize;

use crate::{
    features::{
        common::{db_pool, map_db_error, ApiError},
        specs::repository::SpecsRepository,
    },
    infrastructure::db::models::GameSpecSummary,
    AppState,
};

#[derive(Debug, Serialize)]
pub struct ListSpecsResponse {
    pub items: Vec<GameSpecSummary>,
}

pub async fn handler(State(state): State<AppState>) -> Result<Json<ListSpecsResponse>, ApiError> {
    let pool = db_pool(&state)?;
    let repo = SpecsRepository::new(pool.clone());
    let items = repo.list_specs().await.map_err(map_db_error)?;

    Ok(Json(ListSpecsResponse { items }))
}
