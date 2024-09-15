use std::rc::{Rc, Weak};
use std::cell::RefCell;

use crate::logic::Board;
use crate::shared::Position;

#[derive(Debug)]
pub struct Piece {
    pub code: String,
    pub player: String,
    pub board: Weak<RefCell<Board>>,
}

impl Piece {
    pub fn new(code: String, player: String, board: Weak<RefCell<Board>>) -> Self {
        Piece {
            code,
            player,
            board,
        }
    }

    pub fn get_board(&self) -> Option<Rc<RefCell<Board>>> {
        self.board.upgrade()
    }
}

impl Piece {
    // TODO: Position and action!!
    pub fn calculate_moves(&self) -> Option<Vec<Position>> {
        let board = self.get_board();

        if board.is_none() {
            return None
        }

        let board = board.unwrap();
        let board = board.borrow();

        match board.blueprints.get(&self.code) {
            Some(blueprint) => {
                blueprint.calculate_moves(&board)
            },
            None => None
        }
    }
}
