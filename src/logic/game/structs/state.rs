use std::collections::HashMap;
use crate::shared::{Effect, Position};
use crate::logic::Piece;

use crate::logic::GamePhase;

#[derive(Debug, Clone)]
pub struct GameState {
    // Pieces in the game are stored in a hashmap for quick lookup.
    pub pieces: HashMap<Position, Piece>,

    // Current turn is stored as a cursor to the `turn_order` vector.
    pub current_turn: u8,

    // Available moves are stored as a hashmap of position -> effect.
    // Effects are a set of board changes to be made when a move is executed.
    pub available_moves: Option<HashMap<Position, Effect>>,

    // Current phase of the game
    pub phase: GamePhase,
}