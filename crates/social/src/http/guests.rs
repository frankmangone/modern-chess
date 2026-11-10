use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;

use super::error::ApiError;
use super::{SignedIn, SocialState};
use crate::guests::{create_guest, random_number};

/// `POST /guests`: a new guest user and the token that proves it.
pub async fn create(
    State(state): State<SocialState>,
) -> Result<(StatusCode, Json<SignedIn>), ApiError> {
    let now = Utc::now();
    let user = create_guest(&state.users, now, random_number)
        .await
        .map_err(ApiError::internal)?;
    let token = state.issuer.issue(&user, now).map_err(ApiError::internal)?;
    Ok((StatusCode::CREATED, Json(SignedIn { user, token })))
}
