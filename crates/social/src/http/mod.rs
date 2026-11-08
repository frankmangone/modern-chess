//! The social module's HTTP surface. The app mounts [`router`] under `/api/social`.

mod error;
mod guests;
mod me;

use std::sync::Arc;

use axum::extract::FromRef;
use axum::routing::{get, post};
use axum::Router;
use modern_chess_kernel::TokenVerifier;
use sqlx::PgPool;

use crate::store::UserRepo;
use crate::TokenIssuer;

/// Everything the handlers share.
#[derive(Clone)]
pub struct SocialState {
    pub users: UserRepo,
    pub issuer: Arc<TokenIssuer>,
    verifier: TokenVerifier,
}

impl SocialState {
    /// The verifier is built from the issuer's own public key, so `/me` accepts exactly the
    /// tokens this app issues.
    pub fn new(pool: PgPool, issuer: Arc<TokenIssuer>) -> Self {
        let verifier = TokenVerifier::from_public_pem(issuer.public_pem())
            .expect("the issuer's own public key is valid");
        Self {
            users: UserRepo::new(pool),
            issuer,
            verifier,
        }
    }
}

impl FromRef<SocialState> for TokenVerifier {
    fn from_ref(state: &SocialState) -> TokenVerifier {
        state.verifier.clone()
    }
}

pub fn router(state: SocialState) -> Router {
    Router::new()
        .route("/guests", post(guests::create))
        .route("/me", get(me::show))
        .with_state(state)
}
