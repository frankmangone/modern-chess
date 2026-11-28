//! The game module's HTTP surface. Handlers only translate between HTTP and the domain and
//! store; rules and queries live elsewhere. The app mounts [`router`] under `/api/game`.

mod catalog;
mod drafts;
mod error;
mod images;
mod lobbies;
mod play;
mod review;

use axum::extract::{DefaultBodyLimit, FromRef};
use axum::routing::{get, post, put};
use axum::Router;
use modern_chess_kernel::TokenVerifier;
use sqlx::PgPool;

use crate::store::{
    CommandRepo, InstanceRepo, PageRepo, RecordRepo, SpecRepo, UserGameRepo, VariantRepo,
};
use crate::{Engine, Publisher, Staff};
use images::ImageUrls;

/// Everything the handlers share.
#[derive(Clone)]
pub struct GameState {
    pub specs: SpecRepo,
    pub pages: PageRepo,
    pub variants: VariantRepo,
    pub instances: InstanceRepo,
    pub records: RecordRepo,
    pub commands: CommandRepo,
    pub user_games: UserGameRepo,
    /// The only way the server runs a game's rules: in a sandbox (see `Engine`).
    pub engine: Engine,
    /// Who may do what only staff may do: the list the server started with, and nothing else.
    pub staff: Staff,
    verifier: TokenVerifier,
    images: ImageUrls,
}

impl GameState {
    /// `verifier` holds the public half of the signing key. That is all this module ever
    /// learns about who is calling: it cannot sign, and it never asks the social module.
    pub fn new(pool: PgPool, verifier: TokenVerifier) -> Self {
        let engine = Engine::new();
        Self {
            specs: SpecRepo::new(pool.clone()).with_engine(engine.clone()),
            engine,
            pages: PageRepo::new(pool.clone()),
            variants: VariantRepo::new(pool.clone()),
            instances: InstanceRepo::new(pool.clone()),
            records: RecordRepo::new(pool.clone()),
            commands: CommandRepo::new(pool.clone()),
            user_games: UserGameRepo::new(pool),
            staff: Staff::default(),
            verifier,
            images: ImageUrls::default(),
        }
    }

    /// Runs rules with another engine than the default sandbox (another budget).
    pub fn with_engine(mut self, engine: Engine) -> Self {
        self.specs = self.specs.with_engine(engine.clone());
        self.engine = engine;
        self
    }

    /// Who is staff. Nobody is, until this says so.
    pub fn with_staff(mut self, staff: Staff) -> Self {
        self.staff = staff;
        self
    }

    /// Hands games in and lets staff look at them: the engine, the staff list and the store.
    pub fn publisher(&self) -> Publisher {
        Publisher::new(
            self.user_games.clone(),
            self.engine.clone(),
            self.staff.clone(),
        )
    }

    /// Where stored image keys can be loaded from, such as `http://localhost:9000/bucket`.
    pub fn with_image_base(mut self, base: &str) -> Self {
        self.images = ImageUrls::new(base);
        self
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
        .route("/catalog/{slug}/record", get(catalog::record))
        .route("/catalog/{slug}/lobbies", post(lobbies::create))
        .route("/instances/{id}", get(play::show))
        .route("/instances/{id}/commands", post(play::command))
        .route("/instances/{id}/join", post(lobbies::join))
        .route("/drafts", get(drafts::list).post(drafts::create))
        .route(
            "/drafts/{id}",
            get(drafts::show).put(drafts::change).delete(drafts::delete),
        )
        .route(
            "/drafts/{id}/share",
            put(drafts::share).delete(drafts::unshare),
        )
        .route("/review", get(review::queue))
        .route("/review/{id}", get(review::show))
        .route("/review/{id}/approve", post(review::approve))
        .route("/review/{id}/reject", post(review::reject))
        .route("/my-games", get(drafts::filed))
        .route("/games/{id}/revise", post(drafts::revise))
        .route("/drafts/{id}/publish", post(drafts::publish))
        .route("/drafts/{id}/withdraw", post(drafts::withdraw))
        .route("/shared/{token}", get(drafts::open_shared))
        // A draft's texts are limited to 160 KiB; JSON escaping can double that. Nothing here
        // needs more, and the default (2 MiB) would let a request be far bigger than any draft.
        .layer(DefaultBodyLimit::max(512 * 1024))
        .with_state(state)
}
