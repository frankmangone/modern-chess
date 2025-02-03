use std::collections::{HashSet, HashMap};
use std::rc::Rc;
use std::cell::RefCell;

use crate::specs::{BoardSpec, PieceSpec};
use crate::shared::{Position, ExtendedPosition, PositionOccupant, Move, into_position};
use crate::logic::Piece;

use super::blueprint::PieceBlueprint;

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
    /// Calculate the moves that a piece can make.
    pub fn calculate_moves(&self, player: &String, position: &Position) -> Option<Vec<Move>> {
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

    /// Execute a move on the board.
    pub fn execute_move(&mut self, player: &String, position: &Position) {
        let piece = self.pieces.get_mut(position).unwrap();
        
        // TODO: Execute move.
    }

    /// Checks whether if a position is valid by examining out-of-bounds conditions
    /// and disabled positions.
    pub fn is_position_valid(&self, position: &ExtendedPosition) -> bool {
        for i in 0..position.len() {
            if position[i] < 0 || position[i] > self.dimensions[i] as i16 - 1i16 {
                // Value is outside of range.
                return false
            }

            if self.disabled_positions.contains(&into_position(position)) {
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
    pub fn position_occupant(&self, position: &Position, player: &String) -> Option<PositionOccupant> {
        let piece = self.piece_at_position(position);

        match piece {
            None => None,
            Some(p) => {
                let piece = p.code.clone();
                let player = player.clone();
                Some(PositionOccupant { piece, player })
            }
        }
    }
}