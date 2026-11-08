//! The game module: game specs, the tunable page of each game, lobbies and instances.
//!
//! Pure domain types live in `domain`. Nothing here talks to a database or to HTTP, and
//! nothing here knows the social module exists: users are only ever a `UserId` and a
//! `Handle` from the kernel.

mod domain;
pub mod http;
pub mod seed;
pub mod store;

pub use domain::*;
pub use http::{router, GameState};
pub use store::{migrate, MIGRATOR};
