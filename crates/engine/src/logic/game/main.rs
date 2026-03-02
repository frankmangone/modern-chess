use std::collections::{HashMap, HashSet};

use serde_json;
use crate::specs::GameSpec;
use crate::shared::{into_string, Position, EMPTY, NOT_EMPTY, POSITION, STATE};

use super::{Piece, Board, ConditionDef, GameError, GameState, GamePhase, GameTransition, MoveRecord};
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

    // Draw condition settings, populated from spec at build time.
    pub repetition_count: Option<u8>,
    pub fifty_move_halfmoves: Option<u16>,
    pub fifty_move_pawn_codes: Vec<String>,
    /// Each inner Vec is a sorted piece-code multiset. A player whose full piece
    /// multiset matches any entry has insufficient mating material.
    pub insufficient_material: Vec<Vec<String>>,
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

        // Pre-sort each insufficient-material set so runtime comparisons are O(n log n).
        let insufficient_material = spec.draw_conditions.insufficient_material
            .into_iter()
            .map(|mut v| { v.sort(); v })
            .collect();

        Game {
            turn_order,
            name: spec.name,
            conditions,
            players,
            leader: spec.leader,
            repetition_count: spec.draw_conditions.repetition_count,
            fifty_move_halfmoves: spec.draw_conditions.fifty_move_halfmoves,
            fifty_move_pawn_codes: spec.draw_conditions.fifty_move_pawn_codes,
            insufficient_material,
            state: GameState {
                pieces,
                current_turn,
                available_moves: None,
                phase: GamePhase::Idle,
                history: Vec::new(),
                position_hashes: Vec::new(),
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

    /// Builds a deterministic string key encoding the full position:
    /// active player + all pieces sorted by position, with their code, player,
    /// total_moves, and state flags. Used for repetition detection.
    fn position_key(&self) -> String {
        let mut pieces: Vec<_> = self.state.pieces.iter().collect();
        pieces.sort_by(|a, b| a.0.cmp(b.0));

        let pieces_str: String = pieces.iter().map(|(pos, p)| {
            let pos_s = pos.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",");
            let mut flags: Vec<_> = p.state.iter().collect();
            flags.sort_by(|a, b| a.0.cmp(b.0));
            let flags_s = flags.iter().map(|(k, v)| {
                let v_s = match v {
                    crate::logic::PieceState::Blank       => "B".to_string(),
                    crate::logic::PieceState::Uint(n)     => format!("U{n}"),
                    crate::logic::PieceState::String(s)   => format!("S{s}"),
                };
                format!("{k}={v_s}")
            }).collect::<Vec<_>>().join(";");
            format!("[{pos_s}:{piece}:{player}:{mv}:{flags_s}]",
                piece  = p.code,
                player = p.player,
                mv     = (p.total_moves > 0) as u8,
            )
        }).collect();

        format!("{}|{}", self.current_player(), pieces_str)
    }

    /// Records the current position and checks all configured draw conditions.
    /// Returns `true` and sets `GameOver { winner: None }` if a draw is detected.
    fn check_draws(&mut self) -> bool {
        let key = self.position_key();
        self.state.position_hashes.push(key.clone());

        // Repetition draw.
        if let Some(threshold) = self.repetition_count {
            let count = self.state.position_hashes.iter().filter(|h| *h == &key).count();
            if count >= threshold as usize {
                self.state.phase = GamePhase::GameOver { winner: None };
                return true;
            }
        }

        // Fifty-move rule (N consecutive half-moves with no pawn push and no capture).
        if let Some(halfmoves) = self.fifty_move_halfmoves {
            let h = &self.state.history;
            if h.len() >= halfmoves as usize {
                let recent = &h[h.len() - halfmoves as usize..];
                let no_reset = recent.iter().all(|r| {
                    r.action != "CAPTURE"
                        && !self.fifty_move_pawn_codes.contains(&r.piece_code)
                });
                if no_reset {
                    self.state.phase = GamePhase::GameOver { winner: None };
                    return true;
                }
            }
        }

        // Insufficient material: draw when every player's piece set matches a configured entry.
        if !self.insufficient_material.is_empty() {
            let all_insufficient = self.players.iter().all(|player| {
                let mut player_pieces: Vec<String> = self.state.pieces.values()
                    .filter(|p| &p.player == player)
                    .map(|p| p.code.clone())
                    .collect();
                player_pieces.sort();
                self.insufficient_material.iter().any(|entry| player_pieces == *entry)
            });
            if all_insufficient {
                self.state.phase = GamePhase::GameOver { winner: None };
                return true;
            }
        }

        false
    }

    /// Determines whether the game is over after a move/transform and updates the phase.
    pub fn check_game_over(&mut self) {
        if self.check_draws() {
            return;
        }
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

    /// Serializes the mutable runtime state to a JSON string.
    /// The spec-derived structure (blueprints, board, players, etc.) is not included;
    /// restore by calling `restore_state` on a `Game` built from the same spec.
    pub fn save_state(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.state)
    }

    /// Replaces the current game state with one previously produced by `save_state`.
    /// `available_moves` will be `None` after restore; call `CalculateMoves` to repopulate it.
    pub fn restore_state(&mut self, json: &str) -> Result<(), serde_json::Error> {
        self.state = serde_json::from_str(json)?;
        Ok(())
    }

    /// Returns the full move history as a read-only slice.
    pub fn history(&self) -> &[MoveRecord] {
        &self.state.history
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
