//! The game module's HTTP surface. Handlers only translate between HTTP and the domain and
//! store; rules and queries live elsewhere. The app mounts [`router`] under `/api/game`.

mod catalog;
mod error;

use axum::routing::get;
use axum::Router;
use sqlx::PgPool;

use crate::store::{CommandRepo, InstanceRepo, PageRepo, SpecRepo};

/// Everything the handlers share.
#[derive(Clone)]
pub struct GameState {
    pub specs: SpecRepo,
    pub pages: PageRepo,
    pub instances: InstanceRepo,
    pub commands: CommandRepo,
}

impl GameState {
    pub fn new(pool: PgPool) -> Self {
        Self {
            specs: SpecRepo::new(pool.clone()),
            pages: PageRepo::new(pool.clone()),
            instances: InstanceRepo::new(pool.clone()),
            commands: CommandRepo::new(pool),
        }
    }
}

pub fn router(state: GameState) -> Router {
    Router::new()
        .route("/catalog", get(catalog::list))
        .route("/catalog/{slug}", get(catalog::detail))
        .with_state(state)
}
