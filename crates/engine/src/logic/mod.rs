//! The engine's logic, one folder per idea:
//!
//! - `places`: the board, its zones, a named place, and the holders that keep many pieces.
//! - `pieces`: what a piece is, how it moves (blueprints), and what attacks what.
//! - `rules`: what a spec's rules compile into (steps, tests), the end checks, repetition and
//!   elimination.
//! - `play`: answering the choices of a turn, and the ways a turn offers.
//! - `game`: the `Game` itself: its rules, its state, building it from a spec, and whose turn it is.

pub(crate) mod game;
pub mod pieces;
pub mod places;
pub mod play;
pub mod rules;

pub use game::error::GameError;
pub use game::state::move_record::MoveRecord;
pub use game::state::view::PiecesView;
pub use game::state::GameState;
pub use game::{Game, Rules};
pub use pieces::piece::{Piece, PieceState, Removal};
pub use places::board::graph::{Graph, LabelId, Labels, NodeId, Offset};
pub use places::board::Board;
pub use places::holders::BagContents;
pub use places::zones::Zone;
pub use play::phase::GamePhase;
pub use play::transition::GameTransition;
pub use play::ways::{ActionOption, Answer, Choice, ChoiceKind, PlaceOption, Way};
