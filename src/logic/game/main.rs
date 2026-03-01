use std::collections::{HashMap, HashSet};

use crate::specs::GameSpec;
use crate::shared::{into_string, Position, EMPTY, NOT_EMPTY, POSITION, STATE};

use super::{Piece, Board, ConditionDef, GameError, GameState, GamePhase, GameTransition};
use crate::logic::blueprint::PieceBlueprint;

#[derive(Debug, Clone)]
pub struct Game {
    // Name of the game, for identification purposes only.
    pub name: String,

    // `conditions` contains the custom conditions for the game.
    pub conditions: HashMap<String, ConditionDef>,

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

    // The piece code that must not be in check (e.g. "KING"). None = no GameOver detection.
    pub leader: Option<String>,
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

        // Process custom conditions.
        let mut conditions: HashMap<String, ConditionDef> = HashMap::new();
        for condition in spec.conditions {
            conditions.insert(condition.code.clone(), ConditionDef {
                r#type: condition.r#type,
                check: condition.check,
            });
        }

        Game {
            turn_order,
            name: spec.name,
            conditions,
            players,
            leader: spec.leader,
            state: GameState {
                pieces,
                current_turn,
                available_moves: None,
                phase: GamePhase::Idle,
            },
            board,
            blueprints,
        }
    }

    // ---------------------------------------------------------------------
    // Main transition function
    // ---------------------------------------------------------------------

    pub fn transition(&mut self, transition: GameTransition) -> Result<(), GameError> {
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


    pub fn next_turn(&mut self) {
        let new_turn = self.state.current_turn + 1;

        if new_turn >= self.turn_order.len() as u8 {
            self.state.current_turn = 0;
        } else {
            self.state.current_turn = new_turn;
        }

        // Tick duration-tracked state flags on every piece.
        for piece in self.state.pieces.values_mut() {
            piece.tick_state_flags();
        }
    }

    pub fn clear_moves(&mut self) {
        self.state.available_moves = None;
    }

    pub fn current_player(&self) -> String {
        self.turn_order[self.state.current_turn as usize].clone()
    }

    /// Finds the piece at a given position. If no piece is present, return None.
    pub fn piece_at_position(&self, position: &Position) -> Option<Piece> {
        self.state.pieces.get(position).cloned()
    }

    /// Returns the set of positions threatened by all pieces belonging to `attacker`.
    pub fn attacked_by(&self, attacker: &str) -> HashSet<Position> {
        Self::attacked_by_pieces(attacker, &self.state.pieces, &self.board, &self.blueprints)
    }

    /// Returns the player who acted just before the current turn.
    pub fn previous_player(&self) -> String {
        let len = self.turn_order.len() as u8;
        let idx = (self.state.current_turn + len - 1) % len;
        self.turn_order[idx as usize].clone()
    }

    /// Computes the attack set for `attacker` using an explicit pieces map (for simulation).
    fn attacked_by_pieces(
        attacker: &str,
        pieces: &HashMap<Position, Piece>,
        board: &Board,
        blueprints: &HashMap<String, PieceBlueprint>,
    ) -> HashSet<Position> {
        pieces.iter()
            .filter(|(_, p)| p.player == attacker)
            .filter_map(|(pos, p)| blueprints.get(&p.code).map(|bp| (pos, bp)))
            .flat_map(|(pos, bp)| bp.calculate_threats_with(attacker, pos, pieces, board))
            .collect()
    }

    /// Returns true if the current player's leader is in check given a simulated pieces map.
    fn leader_in_check_for_pieces(&self, pieces: &HashMap<Position, Piece>) -> bool {
        let Some(ref royal) = self.leader else { return false; };
        let player = self.current_player();
        let king_pos = pieces.iter()
            .find(|(_, p)| p.player == player && &p.code == royal)
            .map(|(pos, _)| pos.clone());
        let Some(king_pos) = king_pos else { return false; };
        self.players.iter()
            .filter(|p| **p != player)
            .any(|opp| Self::attacked_by_pieces(opp, pieces, &self.board, &self.blueprints).contains(&king_pos))
    }

    /// Returns true if the current player's leader is currently in check.
    pub fn leader_in_check(&self) -> bool {
        self.leader_in_check_for_pieces(&self.state.pieces)
    }

    /// Returns true if the current player has at least one legal move (one that does not
    /// leave their leader in check). Short-circuits on the first legal move found.
    pub fn any_legal_moves(&self) -> bool {
        let player = self.current_player();
        self.state.pieces.iter()
            .filter(|(_, piece)| piece.player == player)
            .any(|(pos, piece)| {
                let Some(bp) = self.blueprints.get(&piece.code) else { return false; };
                let Some(moves) = bp.calculate_moves(piece, pos, self) else { return false; };
                moves.values().any(|effect| {
                    let mut sim = self.state.pieces.clone();
                    for change in &effect.board_changes {
                        match &change.piece {
                            Some(p) => { sim.insert(change.position.clone(), p.clone()); },
                            None => { sim.remove(&change.position); },
                        }
                    }
                    !self.leader_in_check_for_pieces(&sim)
                })
            })
    }

    /// Determines whether the game is over after a move/transform and updates the phase.
    pub fn check_game_over(&mut self) {
        if !self.any_legal_moves() {
            let winner = if self.leader_in_check() {
                Some(self.previous_player())
            } else {
                None
            };
            self.state.phase = GamePhase::GameOver { winner };
        } else {
            self.state.phase = GamePhase::Idle;
        }
    }

    pub fn check_position_condition(&self, position: &Position, condition: &String) -> bool {
        let maybe_piece = self.piece_at_position(position);

        // Check standard conditions.
        match maybe_piece {
            Some(_) => {
                if condition == &NOT_EMPTY { return true; }
            }
            None => {
                if condition == &EMPTY { return true; }
            }
        };

        // Check custom conditions.
        // First, fetch condition from spec-parsed custom conditions.
        let Some(condition_def) = self.conditions.get(condition) else { return false; };

        // Condition definition has a type and a HashSet of values that match that condition.
        // We need to check for inclusion appropriately based on the type, converting the inclusion search value
        // to a string if necessary (and appropriately as well).
        let condition_value = match condition_def.r#type.as_str() {
            // Positional conditions need to be serialized to a string.
            POSITION => &into_string(position),

            // State conditions are already state strings.
            STATE => condition,

            // Unknown condition type; checking is not possible.
            _ => return false
        };

        // Conditions are player-specific, so we first need to fetch the HashSet for the current player.
        let current_player = self.current_player();
        let Some(player_condition_def) = condition_def.check.get(&current_player) else { return false; };

        player_condition_def.contains(condition_value)
    }
}
