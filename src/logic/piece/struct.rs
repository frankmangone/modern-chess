use std::rc::{Rc, Weak};
use std::cell::RefCell;

use crate::logic::Board;
use crate::specs::PieceSpec;

#[derive(Debug)]
pub struct Piece {
    pub code: String,
    pub board: Weak<RefCell<Board>>,
}

impl Piece {
    pub fn from_spec(spec: PieceSpec, board: Weak<RefCell<Board>>) -> Self {
        Piece {
            code: spec.code,
            board,
        }
    }

    pub fn get_board(&self) -> Option<Rc<RefCell<Board>>> {
        self.board.upgrade()
    }
}
