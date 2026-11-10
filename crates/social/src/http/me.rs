use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use modern_chess_kernel::CurrentUser;
use serde::Serialize;

use super::error::ApiError;
use super::SocialState;
use crate::accounts::delete_account;
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

/// `DELETE /me`: deletes the caller's account. Here the token stops working at once, because a
/// deleted user is no user. The game process checks tokens offline and cannot know, so it keeps
/// accepting that token until it expires (tokens cannot be revoked). Finished matches stay.
pub async fn delete(
    State(state): State<SocialState>,
    caller: CurrentUser,
) -> Result<StatusCode, ApiError> {
    let deleted = delete_account(&state.accounts, caller.id, Utc::now())
        .await
        .map_err(ApiError::internal)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::Unauthorized)
    }
}
