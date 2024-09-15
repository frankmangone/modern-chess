use std::rc::Rc;
use std::cell::RefCell;

use crate::specs::GameSpec;

use crate::logic::Board;

#[derive(Debug)]
pub struct Game {
    pub name: String,
    pub players: Vec<String>,
    pub board: Rc<RefCell<Board>>
}

impl Game {
    pub fn from_spec(spec: GameSpec) -> Game {
        let players: Vec<String> = spec.players.into_iter().map(|x| x.name).collect();
        
        // Board is created as a smart pointer so that it can later be passed as a reference
        // to each piece without creating circular references.
        let board = Board::from_spec(spec.board);

        // Add pieces to the board
        for piece_spec in spec.pieces {
            board.borrow_mut().add_piece(piece_spec, &board);
        }

        Game {
            name: spec.name,
            players,
            board,
        }
    }
}