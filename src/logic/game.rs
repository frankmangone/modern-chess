use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use crate::specs::GameSpec;
use crate::logic::Board;
use crate::shared::{Position, Effect};

use super::Piece;

pub enum GameState {
    Idle,
    Moving,
    // TODO: If more game states should be possible, we need to pull them from specs as a `Custom` state.
    // This is not an easy task though. This will take some thinking.
}

#[derive(Debug)]
pub struct Game {
    // Name of the game, for identification purposes only.
    pub name: String,

    // A list of available players. Doubles up as a sort of dynamic enum.
    pub players: Vec<String>,

    // The board contains all the pieces and specifies allowed positions.
    pub board: Rc<RefCell<Board>>,

    // Turns are just a vector specifying the order in which players play,
    // and a cursor is kept to know the current turn.
    pub turn_order: Vec<String>,
    
    // Game-state-related stuff is also kept in the `Game` struct, in a sort of controller
    // style.
    pub current_turn: u8,

    // TODO: Separate available moves and effects.
    pub available_moves: Option<HashMap<Position, Effect>>
}

impl Game {
    // ---------------------------------------------------------------------
    // Associated fns to parse spec
    // ---------------------------------------------------------------------
    pub fn from_spec(spec: GameSpec) -> Self {
        // Process turn information.
        let turn_order = spec.turns.order;
        let current_turn = spec.turns.start_at;

        // Board is created as a smart pointer so that it can later be passed as a reference
        // to each piece without creating circular references.
        let board = Board::from_spec(spec.board, spec.pieces);

        // Process player information.
        let mut players: Vec<String> = Vec::new();

        for player in spec.players.into_iter() {
            // Store players' names (identifiers).
            players.push(player.name.clone());
            
            // Add pieces to the board, based on the starting positions for each player.
            for starting_positions in player.starting_positions {
                let piece_code = starting_positions.piece;

                for position in starting_positions.positions {
                    board.borrow_mut().add_piece(
                        Rc::new(Piece::new(
                            piece_code.clone(),
                            player.name.clone(),
                            Rc::downgrade(&board)
                        )),
                        position
                    )
                }
            }
        }

        Game {
            turn_order,
            current_turn,
            name: spec.name,
            players,
            board,
            available_moves: Option::None,
        }
    }

    // ---------------------------------------------------------------------
    // Turn-related associated fns
    // ---------------------------------------------------------------------
    pub fn next_turn(&mut self) {
        let new_turn = self.current_turn + 1;

        if new_turn > self.turn_order.len() as u8 {
            self.current_turn = 0;
        } else {
            self.current_turn = new_turn
        }
    }

    pub fn current_player(&self) -> String {
        self.turn_order[self.current_turn as usize].clone()
    }

    /// Calculate moves for a specified position.
    /// Move calculation can only happen for the player that's currently playing.
    pub fn calculate_moves(&mut self, position: Position) -> () {
        let index = self.current_turn as usize;
        let current_player = self.turn_order[index].clone();
        let piece_owner = self.board.borrow().piece_at_position(&position);

        if piece_owner.is_some() && piece_owner.unwrap().player == current_player {
            self.available_moves = self.board.borrow().calculate_moves(&current_player, &position);
        } else {
            self.available_moves = None;
        }
    }

    /// Execute a move that's in the `available_moves` vector.
    pub fn execute_move(&mut self, position: Position) -> () {
        println!("{:?}", self.available_moves);

        if self.available_moves.is_none() {
            // TODO: Some sort of error log maybe?
            return;
        }

        let effect = self.available_moves.as_ref().unwrap().get(&position);

        println!("Effect: {:?}", effect);

        match effect {
            Some(effect) => {
                // Execute move's effect.
                self.board.borrow_mut().execute_effect(effect);
                
                // Clear available moves and advance turn
                self.available_moves = None;
                self.next_turn();
            },
            // TODO: Some sort of error log maybe?
            None => ()
        }

        // self.board.borrow_mut().execute_move(&current_player, &position);
    }

    // Determines the action state of the game
    // FIXME: Deprecated??
    // fn get_board_state(&self) -> GameState {
    //     match self.available_moves {
    //         None => GameState::Idle,
    //         Some(_) => GameState::Moving,
    //     }
    // }
}
