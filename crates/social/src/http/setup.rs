//! The set-up a new account goes through: choosing a username now, and more steps later.

use axum::extract::State;
use axum::Json;
use chrono::Utc;
use modern_chess_kernel::CurrentUser;
use serde::Deserialize;

use super::error::ApiError;
use super::{SignedIn, SocialState};
use crate::handles::choose;
use crate::store::StoreError;
use crate::User;

#[derive(Deserialize)]
pub struct ChooseHandle {
    handle: String,
}

/// The caller's user, when they have a set-up to go through. Anyone else may not change their
/// username here: renaming after the set-up is a different decision, not made yet.
async fn pending_user(state: &SocialState, caller: &CurrentUser) -> Result<User, ApiError> {
    let user = state
        .users
        .get(caller.id)
        .await
        .map_err(ApiError::internal)?
        .ok_or(ApiError::Unauthorized)?;
    if user.setup_pending {
        Ok(user)
    } else {
        Err(ApiError::Rejected {
            code: "no_setup",
            message: "There is nothing to set up.",
        })
    }
}

/// `PUT /me/handle`: chooses a username during the set-up. The username is in the token, so the
/// answer carries a new one, in the same shape as signing in.
pub async fn choose_handle(
    State(state): State<SocialState>,
    caller: CurrentUser,
    Json(request): Json<ChooseHandle>,
) -> Result<Json<SignedIn>, ApiError> {
    let mut user = pending_user(&state, &caller).await?;
    let handle = choose(&request.handle).map_err(|problem| ApiError::Rejected {
        code: problem.code(),
        message: problem.message(),
    })?;
    match state.users.set_handle(user.id, &handle).await {
        Ok(()) => {}
        Err(StoreError::HandleTaken) => return Err(ApiError::Taken),
        Err(StoreError::UserMissing) => return Err(ApiError::Unauthorized),
        Err(other) => return Err(ApiError::internal(other)),
    }
    user.handle = handle;
    let account = state
        .accounts
        .is_account(user.id)
        .await
        .map_err(ApiError::internal)?;
    let token = state
        .issuer
        .issue(&user, account, Utc::now())
        .map_err(ApiError::internal)?;
    Ok(Json(SignedIn { user, token }))
}

/// `POST /me/setup`: ends the set-up. Asking again after it ended is not an error.
pub async fn finish(
    State(state): State<SocialState>,
    caller: CurrentUser,
) -> Result<Json<SignedIn>, ApiError> {
    let mut user = state
        .users
        .get(caller.id)
        .await
        .map_err(ApiError::internal)?
        .ok_or(ApiError::Unauthorized)?;
    if user.setup_pending {
        state
            .users
            .finish_setup(user.id)
            .await
            .map_err(ApiError::internal)?;
        user.setup_pending = false;
    }
    let account = state
        .accounts
        .is_account(user.id)
        .await
        .map_err(ApiError::internal)?;
    let token = state
        .issuer
        .issue(&user, account, Utc::now())
        .map_err(ApiError::internal)?;
    Ok(Json(SignedIn { user, token }))
}
