use std::collections::{HashSet, HashMap};
use std::rc::{Rc, Weak};
use std::cell::RefCell;

use crate::specs::{BoardSpec, PieceSpec};
use crate::shared::{Position, ExtendedPosition, PositionOccupant};
use crate::logic::Piece;

use super::piece_blueprint::PieceBlueprint;

/// A `Board` is a representation of everything board-related. Of course,
/// boards contain pieces, and have a shape that establishes which positions
/// are viable.
#[derive(Debug)]
pub struct Board {
    // Board shape specifications
    pub dimensions: Vec<i16>,
    pub disabled_positions: HashSet<ExtendedPosition>,

    // `blueprints` allow for calculation of piece movements.
    pub blueprints: HashMap<String, PieceBlueprint>, 

    // `pieces` is a list of the actual pieces existing in the board.
    pub pieces: HashMap<Position, Rc<Piece>>,
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
    pub fn calculate_moves(&self, player: &String, position: &Position) -> Option<Vec<Position>> {
        let maybe_piece = self.pieces.get(position);

        match maybe_piece {
            Some(piece) => {
                if player != &piece.player {
                    return None
                }

                piece.calculate_moves(player, position)
            }
            None => None
        }
        
    }

    /// Checks whether if a position is valid by examining out-of-bounds conditions
    /// and disabled positions.
    pub fn is_position_valid(&self, position: &ExtendedPosition) -> bool {
        for i in 0..position.len() {
            if position[i] < 0 || position[i] > self.dimensions[i] - 1 {
                // Value is outside of range.
                return false
            }

            if self.disabled_positions.contains(position) {
                // Value is in one of the known disabled positions.
                return false
            }
        }

        true
    }

    /// Finds the piece at a given position. If no piece is present, return None.
    pub fn piece_at_position(&self, position: &Position) -> Option<Rc<Piece>> {
        self.pieces.get(position).cloned()
    }

    /// Determines what's the occupation state of a position. This is player-dependent.
    pub fn position_occupant(&self, position: &Position, player: &String) -> PositionOccupant {
        let piece = self.piece_at_position(position);

        match piece {
            None => PositionOccupant::Empty,
            Some(p) => {
                if &p.player == player {
                    PositionOccupant::Ally(p.code.clone())
                } else {
                    PositionOccupant::Enemy(p.code.clone(), player.clone())
                }
            }
        }
    }
}