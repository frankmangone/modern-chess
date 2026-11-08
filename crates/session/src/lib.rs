//! A playable game and the snapshot a front end draws. Native code (the game server) and the
//! WebAssembly board both use this crate, so they produce identical views.

pub mod session;
pub mod view;

pub use session::Session;
pub use view::{PieceView, Square, Status, View};
