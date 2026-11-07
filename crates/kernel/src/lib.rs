//! Shared vocabulary: identifiers and the token contract. Nothing else belongs here.
//!
//! The game and social modules never depend on each other. The only things they agree on
//! live in this crate: who a user is (`UserId`), what they are called (`Handle`), and the
//! claims a signed token carries (`TokenClaims`).

mod error;
mod handle;
mod token;
mod user_id;

pub use error::DomainError;
pub use handle::Handle;
pub use token::TokenClaims;
pub use user_id::UserId;
