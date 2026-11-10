use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use chrono::{DateTime, Utc};
use modern_chess_kernel::bearer_token;
use serde::Deserialize;

use super::error::ApiError;
use super::{SignedIn, SocialState};
use crate::accounts::sign_in;
use crate::guests::random_number;
use crate::provider::{ExternalIdentity, ProviderError};

#[derive(Deserialize)]
pub struct SessionRequest {
    /// The caller's current guest token, if they have one. When it is good, the account adopts
    /// that guest. A token that is not good is ignored: signing in still works.
    guest_token: Option<String>,
}

/// Who the provider's token in the `Authorization` header belongs to. Shared by the calls that
/// ask for the provider's token itself: signing in, and deleting an account.
pub(super) async fn provider_identity(
    state: &SocialState,
    headers: &HeaderMap,
    now: DateTime<Utc>,
) -> Result<ExternalIdentity, ApiError> {
    let provider = state.provider.as_ref().ok_or(ApiError::NotConfigured)?;
    let credential = bearer_token(headers).ok_or(ApiError::Unauthorized)?;
    provider
        .verify(credential, now)
        .await
        .map_err(|err| match err {
            ProviderError::Invalid => ApiError::Unauthorized,
            other => ApiError::internal(other),
        })
}

/// `POST /session`: trades a sign-in provider's token (as the bearer credential) for one of
/// ours. The body is optional: without one there is simply no guest to adopt. This is the
/// only place a provider's token is ever looked at.
pub async fn create(
    State(state): State<SocialState>,
    headers: HeaderMap,
    request: Option<Json<SessionRequest>>,
) -> Result<Json<SignedIn>, ApiError> {
    let now = Utc::now();
    let identity = provider_identity(&state, &headers, now).await?;
    let guest = request
        .and_then(|Json(request)| request.guest_token)
        .as_deref()
        .and_then(|token| state.verifier.verify(token, now).ok())
        .map(|claims| claims.sub);

    let user = sign_in(
        &state.users,
        &state.accounts,
        &identity,
        guest,
        now,
        random_number,
    )
    .await
    .map_err(ApiError::internal)?;
    let token = state.issuer.issue(&user, now).map_err(ApiError::internal)?;
    Ok(Json(SignedIn { user, token }))
}
