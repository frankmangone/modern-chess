use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;

use crate::{
    features::{
        common::{db_pool, map_db_error, ApiError},
        specs::repository::seed_default_specs,
    },
    infrastructure::db::models::SeededSpecSummary,
    AppState,
};

#[derive(Debug, Serialize)]
pub struct SeedDefaultsResponse {
    pub seeded: Vec<SeededSpecSummary>,
}

pub async fn handler(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<SeedDefaultsResponse>), ApiError> {
    let pool = db_pool(&state)?;
    let seeded = seed_default_specs(pool).await.map_err(map_db_error)?;

    Ok((StatusCode::OK, Json(SeedDefaultsResponse { seeded })))
}
