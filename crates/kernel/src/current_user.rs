use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use serde_json::json;

use crate::{Handle, TokenVerifier, UserId};

/// The caller, as named by a valid `Authorization: Bearer <token>` header.
///
/// Add it to a handler's arguments to require a signed-in user. The app state must be able
/// to hand out a [`TokenVerifier`] (`impl FromRef<State> for TokenVerifier`). Any problem
/// with the credential, whatever it is, rejects the request with a plain 401.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentUser {
    pub id: UserId,
    pub handle: Handle,
}

/// The 401 answer: no detail about what was wrong with the credential.
#[derive(Debug)]
pub struct Unauthorized;

impl IntoResponse for Unauthorized {
    fn into_response(self) -> Response {
        (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer")],
            Json(json!({ "error": "sign in required" })),
        )
            .into_response()
    }
}

impl<S> FromRequestParts<S> for CurrentUser
where
    TokenVerifier: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Unauthorized;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let verifier = TokenVerifier::from_ref(state);
        let token = bearer_token(parts).ok_or(Unauthorized)?;
        let claims = verifier
            .verify(token, Utc::now())
            .map_err(|_| Unauthorized)?;
        Ok(CurrentUser {
            id: claims.sub,
            handle: claims.handle,
        })
    }
}

fn bearer_token(parts: &Parts) -> Option<&str> {
    let value = parts.headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;
    let token = token.trim();
    (scheme.eq_ignore_ascii_case("bearer") && !token.is_empty()).then_some(token)
}
