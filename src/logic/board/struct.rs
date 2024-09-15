use std::collections::{HashSet, HashMap};
use std::rc::Rc;
use std::cell::RefCell;

use crate::specs::BoardSpec;
use crate::shared::Position;
use crate::logic::Piece;

use super::player_piece::PlayerPiece;

/// A `Board` is a representation of everything board-related. Of course,
/// boards contain pieces, and have a shape that establishes which positions
/// are viable.
#[derive(Debug)]
pub struct Board {
    pub dimensions: Vec<u8>,
    pub disabled_positions: HashSet<Position>,
    pub pieces: HashMap<Position, PlayerPiece>
}

// ---------------------------------------------------------------------
// Associated fns to parse spec
// ---------------------------------------------------------------------
impl Board {
    pub fn from_spec(spec: BoardSpec) -> Rc<RefCell<Board>> {
        Rc::new(RefCell::new(Board {
            dimensions: spec.dimensions,
            disabled_positions: spec.disabled_positions,
            pieces: HashMap::new(),
        }))
    }

    pub fn add_piece(&mut self, piece: Rc<Piece>, player: String, position: Position) {
        self.pieces.insert(position, PlayerPiece {
            player,
            piece
        });
    }
}

// ---------------------------------------------------------------------
// Logic-related associated fns
// ---------------------------------------------------------------------
impl Board {
    // pub fn get_moves_at_position
}