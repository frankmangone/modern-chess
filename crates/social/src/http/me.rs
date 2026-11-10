use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use chrono::Utc;
use modern_chess_kernel::CurrentUser;
use serde::Serialize;

use super::error::ApiError;
use super::session::provider_identity;
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

/// `DELETE /me`: deletes an account. It asks for the sign-in provider's token, not ours: a
/// provider token lasts about a minute, so only someone signed in right now can delete. A
/// leftover token of ours, which stays valid for days, cannot. Guests have no account and
/// nothing to delete.
///
/// Here our token for the account stops working at once, because a deleted user is no user.
/// The game process checks tokens offline and cannot know, so it keeps accepting it until it
/// expires (tokens cannot be revoked). Finished matches stay.
pub async fn delete(
    State(state): State<SocialState>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let now = Utc::now();
    let identity = provider_identity(&state, &headers, now).await?;
    let user = state
        .accounts
        .find_user(&identity.provider, &identity.subject)
        .await
        .map_err(ApiError::internal)?
        .ok_or(ApiError::Unauthorized)?;
    let deleted = delete_account(&state.accounts, user, now)
        .await
        .map_err(ApiError::internal)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::Unauthorized)
    }
}
