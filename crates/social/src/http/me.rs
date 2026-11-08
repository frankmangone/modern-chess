use axum::extract::State;
use axum::Json;
use modern_chess_kernel::CurrentUser;
use serde::Serialize;

use super::error::ApiError;
use super::SocialState;
use crate::User;

#[derive(Serialize)]
pub struct MeResponse {
    user: User,
}

/// `GET /me`: the signed-in user. A valid token for a user that no longer exists (a reset
/// database) is treated as no token at all, so the web app signs in as a new guest.
pub async fn show(
    State(state): State<SocialState>,
    caller: CurrentUser,
) -> Result<Json<MeResponse>, ApiError> {
    let user = state
        .users
        .get(caller.id)
        .await
        .map_err(ApiError::internal)?
        .ok_or(ApiError::Unauthorized)?;
    Ok(Json(MeResponse { user }))
}
