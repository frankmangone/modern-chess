//! The social module: users, and later the community.
//!
//! Knows nothing about games. The only thing it shares with the game module is the
//! kernel's vocabulary.

pub mod accounts;
mod domain;
pub mod guests;
pub mod http;
pub mod provider;
pub mod store;
mod token;

pub use domain::User;
pub use http::{router, SocialState};
pub use store::{migrate, MIGRATOR};
pub use token::TokenIssuer;
