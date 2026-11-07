//! The game module's HTTP surface. Handlers only translate between HTTP and the domain and
//! store; rules and queries live elsewhere. The app mounts [`router`] under `/api/game`.

mod catalog;
mod error;
mod lobbies;

use axum::extract::FromRef;
use axum::routing::{get, post};
use axum::Router;
use modern_chess_kernel::TokenVerifier;
use sqlx::PgPool;

use crate::store::{CommandRepo, InstanceRepo, PageRepo, SpecRepo};

/// Everything the handlers share.
#[derive(Clone)]
pub struct GameState {
    pub specs: SpecRepo,
    pub pages: PageRepo,
    pub instances: InstanceRepo,
    pub commands: CommandRepo,
    verifier: TokenVerifier,
}

impl GameState {
    /// `verifier` holds the public half of the signing key. That is all this module ever
    /// learns about who is calling: it cannot sign, and it never asks the social module.
    pub fn new(pool: PgPool, verifier: TokenVerifier) -> Self {
        Self {
            specs: SpecRepo::new(pool.clone()),
            pages: PageRepo::new(pool.clone()),
            instances: InstanceRepo::new(pool.clone()),
            commands: CommandRepo::new(pool),
            verifier,
        }
    }
}

impl FromRef<GameState> for TokenVerifier {
    fn from_ref(state: &GameState) -> TokenVerifier {
        state.verifier.clone()
    }
}

pub fn router(state: GameState) -> Router {
    Router::new()
        .route("/catalog", get(catalog::list))
        .route("/catalog/{slug}", get(catalog::detail))
        .route("/catalog/{slug}/lobbies", post(lobbies::create))
        .route("/instances/{id}", get(lobbies::show))
        .route("/instances/{id}/join", post(lobbies::join))
        .with_state(state)
}
