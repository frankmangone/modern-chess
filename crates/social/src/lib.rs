//! The social module: users, and later the community.
//!
//! Knows nothing about games. The only thing it shares with the game module is the
//! kernel's vocabulary.

mod domain;

pub use domain::User;
