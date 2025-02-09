use std::collections::HashMap;

use crate::specs::GameSpec;
use crate::logic::Board;
use crate::shared::{Effect, Position, EMPTY, NOT_EMPTY};

use super::Piece;
use super::blueprint::PieceBlueprint;

#[derive(Debug, Clone)]
pub enum GamePhase {
    // No piece selected, waiting for player input
    Idle,

    // Piece selected, showing available moves
    Moving { position: Position },

    // Move selected, piece needs transformation
    Transforming { 
        position: Position,
        options: Vec<String>
    }
}

#[derive(Debug)]
pub struct GameState {
    // Pieces in the game are stored in a hashmap for quick lookup.
    pub pieces: HashMap<Position, Piece>,

    // Current turn is stored as a cursor to the `turn_order` vector.
    pub current_turn: u8,

    // Available moves are stored as a hashmap of position -> effect.
    // Effects are a set of board changes to be made when a move is executed.
    pub available_moves: Option<HashMap<Position, Effect>>,

    // Current phase of the game
    pub phase: GamePhase,
}

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
    // Associated fns to parse spec
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
    // Game logic
    // ---------------------------------------------------------------------

    /// Calculate moves for a specified position.
    /// Move calculation can only happen for the player that's currently playing.
    /// TODO: Return Result<(), Error>?
    pub fn calculate_moves(&mut self, position: Position) -> () {
        let maybe_piece = self.state.pieces.get(&position);

        match maybe_piece {
            Some(piece) => {
                if piece.player != self.current_player() {
                    // TODO: Some sort of error log maybe?
                    return;
                }

                match self.blueprints.get(&piece.code) {
                    Some(blueprint) => {
                        self.state.available_moves = blueprint.calculate_moves(&piece, &position, &self);
                        self.state.phase = GamePhase::Moving { 
                             position 
                        };
                    }
                    None => ()
                }
            },
            None => ()
        }
    }

    /// Execute a move that's in the `available_moves` vector.
    /// TODO: Return Result<(), Error>?
    pub fn execute_move(&mut self, position: Position) -> () {
        if self.state.available_moves.is_none() {
            // TODO: Some sort of error log maybe?
            return;
        }

        let effect = self.state.available_moves.as_ref().unwrap().get(&position);

        if effect.is_none() {
            // TODO: Some sort of error log maybe?
            return;
        }

        let effect = effect.unwrap();
        
        // Check if this move requires transformation
        if let Some(modifier) = effect.modifiers.iter().find(|m| m.action == "TRANSFORM") {
            // Store transformation state and return
            if let GamePhase::Moving { position } = &self.state.phase {
                self.state.phase = GamePhase::Transforming {
                    position: position.clone(),
                    options: modifier.options.clone(),
                };
                return;
            }
        }

        // Execute the move if no transformation needed
        self.apply_move_effect(&effect.clone());
        self.next_turn();
        self.clear_moves();
        self.state.phase = GamePhase::Idle;
    }

    // New method to handle transformation
    pub fn execute_transformation(&mut self, piece_code: String) -> Result<(), String> {
        match &self.state.phase {
            GamePhase::Transforming { position, options } => {
                if !options.contains(&piece_code) {
                    return Err("Invalid transformation option".to_string());
                }

                // Create new transformed piece
                let old_piece = self.state.pieces.get(position).unwrap();
                let new_piece = Piece::new(piece_code, old_piece.player.clone());

                // Apply the transformation
                self.state.pieces.remove(position);
                self.state.pieces.insert(position.clone(), new_piece);

                // Reset game state
                self.next_turn();
                self.clear_moves();
                self.state.phase = GamePhase::Idle;
                Ok(())
            },
            _ => Err("Game is not in transformation phase".to_string())
        }
    }

    // Helper method to apply move effects
    fn apply_move_effect(&mut self, effect: &Effect) {
        effect.board_changes.iter().for_each(|change| {
            match &change.piece {
                Some(piece) => {
                    self.state.pieces.insert(
                        change.position.clone(),
                        piece.clone(),
                    );
                },
                None => {
                    self.state.pieces.remove(&change.position);
                },
            }
        });
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
