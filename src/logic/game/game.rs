use std::collections::HashMap;

use crate::specs::GameSpec;
use crate::shared::{Position, EMPTY, NOT_EMPTY};

use super::{Piece, Board, GameState, GamePhase, GameTransition};
use crate::logic::blueprint::PieceBlueprint;

#[derive(Debug)]
pub struct Game {
    // Name of the game, for identification purposes only.
    pub name: String,

    // A list of available players. Doubles up as a sort of dynamic enum.
    pub players: Vec<String>,

    // The `board` stores info about dimensions and disabled positions.
    pub board: Board,

    // `blueprints` allow for calculation of piece movements.
    pub blueprints: HashMap<String, PieceBlueprint>, 

    // `turn_order` is just a vector specifying the order in which players play,
    // and a cursor is kept to know the current turn.
    pub turn_order: Vec<String>,

    // `state` contains the actual game state.
    pub state: GameState,
}

impl Game {
    // ---------------------------------------------------------------------
    // Spec parsing
    // ---------------------------------------------------------------------
    pub fn from_spec(spec: GameSpec) -> Self {
        // Process turn information.
        let turn_order = spec.turns.order;
        let current_turn = spec.turns.start_at;

        // Board is created as a smart pointer so that it can later be passed as a reference
        // to each piece without creating circular references.
        let board = Board::from_spec(spec.board);

        // Create blueprints for each piece & player.
        // TODO: Optimize for pieces that are not direction-dependent.
        let mut blueprints = HashMap::new();

        for piece_spec in spec.pieces {
            blueprints.insert(piece_spec.code.clone(), PieceBlueprint::from_spec(piece_spec.clone(), spec.players.clone()));
        }

        // Process player information.
        let mut players: Vec<String> = Vec::new();
        let mut pieces: HashMap<Position, Piece> = HashMap::new();

        for player in spec.players.into_iter() {
            // Store players' names (identifiers).
            players.push(player.name.clone());
            
            // Add pieces to the board, based on the starting positions for each player.
            for starting_positions in player.starting_positions {
                let piece_code = starting_positions.piece;

                for position in starting_positions.positions {
                    pieces.insert(
                        position, 
                        Piece::new(
                            piece_code.clone(),
                            player.name.clone()
                        )
                    );
                }
            }
        }

        Game {
            turn_order,
            name: spec.name,
            players,
            state: GameState {
                pieces,
                current_turn,
                available_moves: Option::None,
                phase: GamePhase::Idle,
            },
            board,
            blueprints,
        }
    }

    // ---------------------------------------------------------------------
    // Main transition function
    // ---------------------------------------------------------------------

    pub fn transition(&mut self, transition: GameTransition) -> Result<(), String> {
        match transition {
            GameTransition::CalculateMoves { position } => {
                self.calculate_moves(position)
            },
            GameTransition::ExecuteMove { position } => {
                self.execute_move(position)
            },
            GameTransition::Transform { target } => {
                self.transform(target)
            }
        }
    }

    // ---------------------------------------------------------------------
    // Utility functions
    // ---------------------------------------------------------------------


    pub fn next_turn(&mut self) -> () {
        let new_turn = self.state.current_turn + 1;

        if new_turn >= self.turn_order.len() as u8 {
            self.state.current_turn = 0;
        } else {
            self.state.current_turn = new_turn
        }
    }

    pub fn clear_moves(&mut self) -> () {
        self.state.available_moves = Option::None;
    }

    pub fn current_player(&self) -> String {
        self.turn_order[self.state.current_turn as usize].clone()
    }

    /// Finds the piece at a given position. If no piece is present, return None.
    pub fn piece_at_position(&self, position: &Position) -> Option<Piece> {
        self.state.pieces.get(position).cloned()
    }

    pub fn check_position_condition(&self, position: &Position, condition: &String) -> bool {
        let maybe_piece = self.piece_at_position(position);

        // TODO: Improve this logic with custom conditions.
        match maybe_piece {
            Some(_) => {
                if condition == &NOT_EMPTY {
                    true
                } else {
                    false
                }
            }
            None => {
                if condition == &EMPTY {
                    true
                } else {
                    false
                }
            }
        }
    }
}
