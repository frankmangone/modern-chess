//! The social module's HTTP surface. The app mounts [`router`] under `/api/social`.

mod error;
mod guests;
mod me;
mod session;

use std::sync::Arc;

use axum::extract::FromRef;
use axum::routing::{get, post};
use axum::Router;
use modern_chess_kernel::TokenVerifier;
use serde::Serialize;
use sqlx::PgPool;

use crate::provider::ProviderVerifier;
use crate::store::{AccountRepo, UserRepo};
use crate::{TokenIssuer, User};

/// Everything the handlers share.
#[derive(Clone)]
pub struct SocialState {
    pub users: UserRepo,
    pub accounts: AccountRepo,
    pub issuer: Arc<TokenIssuer>,
    verifier: TokenVerifier,
    provider: Option<Arc<ProviderVerifier>>,
}

impl SocialState {
    /// The verifier is built from the issuer's own public key, so `/me` accepts exactly the
    /// tokens this app issues.
    pub fn new(pool: PgPool, issuer: Arc<TokenIssuer>) -> Self {
        let verifier = TokenVerifier::from_public_pem(issuer.public_pem())
            .expect("the issuer's own public key is valid");
        Self {
            users: UserRepo::new(pool.clone()),
            accounts: AccountRepo::new(pool),
            issuer,
            verifier,
            provider: None,
        }
    }

    /// Turns on signing in with an outside provider. Without it `POST /session` answers 503
    /// and guests are all there is.
    pub fn with_provider(mut self, provider: Arc<ProviderVerifier>) -> Self {
        self.provider = Some(provider);
        self
    }
}

/// What a sign-in or sign-up answers: the user, and the token that proves it.
#[derive(Serialize)]
struct SignedIn {
    user: User,
    token: String,
}

impl FromRef<SocialState> for TokenVerifier {
    fn from_ref(state: &SocialState) -> TokenVerifier {
        state.verifier.clone()
    }
}

pub fn router(state: SocialState) -> Router {
    Router::new()
        .route("/guests", post(guests::create))
        .route("/session", post(session::create))
        .route("/me", get(me::show).delete(me::delete))
        .with_state(state)
}
