use std::collections::{HashSet, HashMap};
use std::rc::Rc;
use std::cell::RefCell;

use crate::specs::{BoardSpec, PieceSpec};
use crate::shared::Position;
use crate::logic::Piece;

use super::piece_blueprint::PieceBlueprint;

/// A `Board` is a representation of everything board-related. Of course,
/// boards contain pieces, and have a shape that establishes which positions
/// are viable.
#[derive(Debug)]
pub struct Board {
    // Board shape specifications
    pub dimensions: Vec<u8>,
    pub disabled_positions: HashSet<Position>,

    // `blueprints` allow for calculation of piece movements.
    pub blueprints: HashMap<String, PieceBlueprint>, 

    // `pieces` is a list of the actual pieces existing in the board.
    pub pieces: HashMap<Position, Rc<Piece>>
}

// ---------------------------------------------------------------------
// Associated fns to parse spec
// ---------------------------------------------------------------------
impl Board {
    pub fn from_spec(board_spec: BoardSpec, pieces_spec: Vec<PieceSpec>) -> Rc<RefCell<Board>> {
        let mut blueprints = HashMap::new();


        for piece_spec in pieces_spec {
            blueprints.insert(piece_spec.code.clone(), PieceBlueprint::from_spec(piece_spec));
        }

        Rc::new(RefCell::new(Board {
            dimensions: board_spec.dimensions,
            disabled_positions: board_spec.disabled_positions,
            blueprints,
            pieces: HashMap::new(),
        }))
    }

    pub fn add_piece(&mut self, piece: Rc<Piece>, position: Position) {
        self.pieces.insert(position, piece);
    }
}

// ---------------------------------------------------------------------
// Logic-related associated fns
// ---------------------------------------------------------------------
impl Board {
    pub fn calculate_moves(&self, player: String, position: Position) -> Option<Vec<Position>> {
        let maybe_piece = self.pieces.get(&position);

        match maybe_piece {
            Some(piece) => {
                if player != piece.player {
                    return None
                }

                piece.calculate_moves()
            }
            None => None
        }
        
    }
}