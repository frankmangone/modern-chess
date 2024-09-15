use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use crate::specs::GameSpec;
use crate::logic::Board;

use super::Piece;

#[derive(Debug)]
pub struct Game {
    pub name: String,
    pub players: Vec<String>,
    pub board: Rc<RefCell<Board>>
}

impl Game {
    pub fn from_spec(spec: GameSpec) -> Game {        
        // Board is created as a smart pointer so that it can later be passed as a reference
        // to each piece without creating circular references.
        let board = Board::from_spec(spec.board);

        // Parse known pieces, and associate them with their code.
        let mut known_pieces: HashMap<String, Rc<Piece>> = HashMap::new();

        for piece_spec in spec.pieces {
            let piece = Rc::new(Piece::from_spec(piece_spec.clone(), Rc::downgrade(&board)));
            known_pieces.insert(piece_spec.code, piece);
        }

        // Process player information.
        let mut players: Vec<String> = Vec::new();

        for player in spec.players.into_iter() {
            // Store players' names (identifiers).
            players.push(player.name.clone());
            
            // Add pieces to the board, based on the starting positions for each player.
            for starting_positions in player.starting_positions {
                let piece_code = starting_positions.piece;
                let piece = known_pieces.get(&piece_code).unwrap(); // This should be safe because of spec validation.

                for position in starting_positions.positions {
                    board.borrow_mut().add_piece(
                        piece.clone(),
                        player.name.clone(),
                        position
                    );
                }
            }
        }

        Game {
            name: spec.name,
            players,
            board,
        }
    }
}