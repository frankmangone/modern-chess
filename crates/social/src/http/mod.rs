//! The social module's HTTP surface. The app mounts [`router`] under `/api/social`.

mod discussion;
mod error;
mod guests;
mod handles;
mod me;
mod session;
mod setup;

use std::collections::HashSet;
use std::sync::Arc;

use axum::extract::FromRef;
use axum::routing::{delete, get, post, put};
use axum::Router;
use modern_chess_kernel::{TokenVerifier, UserId};
use serde::Serialize;
use sqlx::PgPool;

use crate::pages::GamePages;
use crate::provider::ProviderVerifier;
use crate::store::{AccountRepo, DiscussionRepo, Limits, UserRepo};
use crate::{TokenIssuer, User};

/// Everything the handlers share.
#[derive(Clone)]
pub struct SocialState {
    pub users: UserRepo,
    pub accounts: AccountRepo,
    pub discussion: DiscussionRepo,
    pub issuer: Arc<TokenIssuer>,
    verifier: TokenVerifier,
    provider: Option<Arc<ProviderVerifier>>,
    pages: Option<Arc<dyn GamePages>>,
    staff: Arc<HashSet<UserId>>,
    limits: Limits,
}

impl SocialState {
    /// The verifier is built from the issuer's own public key, so `/me` accepts exactly the
    /// tokens this app issues.
    pub fn new(pool: PgPool, issuer: Arc<TokenIssuer>) -> Self {
        #[allow(
            clippy::expect_used,
            reason = "the key was just parsed by the issuer; a failure here is a startup bug"
        )]
        let verifier = TokenVerifier::from_public_pem(issuer.public_pem())
            .expect("the issuer's own public key is valid");
        Self {
            users: UserRepo::new(pool.clone()),
            accounts: AccountRepo::new(pool.clone()),
            discussion: DiscussionRepo::new(pool),
            issuer,
            verifier,
            provider: None,
            pages: None,
            staff: Arc::new(HashSet::new()),
            limits: Limits::default(),
        }
    }

    /// Turns on signing in with an outside provider. Without it `POST /session` answers 503
    /// and guests are all there is.
    pub fn with_provider(mut self, provider: Arc<ProviderVerifier>) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Turns on discussion: where to ask whether a game's page exists and is open. Without it the
    /// discussion routes answer 503.
    pub fn with_pages(mut self, pages: Arc<dyn GamePages>) -> Self {
        self.pages = Some(pages);
        self
    }

    /// Who is staff. Nobody is, unless told: this list is the only way to be, and a token cannot
    /// say it.
    pub fn with_staff(mut self, staff: impl IntoIterator<Item = UserId>) -> Self {
        self.staff = Arc::new(staff.into_iter().collect());
        self
    }

    /// How soon a person may post again (the defaults are the product's; tests shorten them).
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    pub fn is_staff(&self, user: UserId) -> bool {
        self.staff.contains(&user)
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
        .route("/me/handle", put(setup::choose_handle))
        .route("/me/setup", post(setup::finish))
        .route("/handles/{name}", get(handles::check))
        .route(
            "/games/{slug}/threads",
            get(discussion::threads).post(discussion::start),
        )
        .route("/threads/{id}", get(discussion::thread))
        .route("/threads/{id}/posts", post(discussion::reply))
        .route("/posts/{id}", delete(discussion::delete_post))
        .with_state(state)
}
