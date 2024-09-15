use std::collections::HashSet;
use std::rc::Rc;
use std::cell::RefCell;

use crate::specs::{PieceSpec, BoardSpec};
use crate::shared::Position;
use crate::logic::Piece;

#[derive(Debug)]
pub struct Board {
    pub dimensions: Vec<u8>,
    pub disabled_positions: HashSet<Position>,
    pub pieces: Vec<Rc<Piece>>,
}

impl Board {
    pub fn from_spec(spec: BoardSpec) -> Rc<RefCell<Board>> {
        Rc::new(RefCell::new(Board {
            dimensions: spec.dimensions,
            disabled_positions: spec.disabled_positions,
            pieces: vec![],
        }))
    }

    pub fn add_piece(&mut self, spec: PieceSpec, board_ref: &Rc<RefCell<Board>>) {
        let piece = Rc::new(Piece::from_spec(spec, Rc::downgrade(board_ref)));
        self.pieces.push(piece);
    }
}