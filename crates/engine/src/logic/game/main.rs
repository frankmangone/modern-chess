use std::collections::{HashMap, HashSet};

use crate::shared::{
    into_string, Position, ALLY, ALLY_ON_FILE, CHECK_COUNT, DROP, EMPTY, ENEMY, NOT_EMPTY,
    OPPONENT_BARE, PIECE_IN_ZONE, POSITION, STATE,
};
use crate::specs::{ConditionSpec, GameSpec, WinConditionSpec};
use serde_json;

use super::{
    Board, ConditionDef, GameError, GamePhase, GameState, GameTransition, MoveRecord, Piece,
};
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

    // Piece codes that must not be in check (e.g. ["KING"]). Empty = no GameOver detection.
    // A player is in check if *any* of their leader pieces is attacked.
    pub leader: Vec<String>,

    // Draw condition settings, populated from spec at build time.
    pub repetition_count: Option<u8>,
    pub fifty_move_halfmoves: Option<u16>,
    pub fifty_move_pawn_codes: Vec<String>,
    /// Each inner Vec is a sorted piece-code multiset. A player whose full piece
    /// multiset matches any entry has insufficient mating material.
    pub insufficient_material: Vec<Vec<String>>,

    /// When `true`, a player with no legal moves always loses regardless of check status.
    pub stalemate_loses: bool,

    /// When `true`, captured pieces enter the capturer's hand and can be dropped.
    pub hand_enabled: bool,

    /// Maps piece code → the code that enters the hand on capture.
    /// `None` means the piece itself (base form) enters the hand.
    pub demotes_to: HashMap<String, Option<String>>,

    /// Win conditions checked after every move, before draw/checkmate detection.
    pub win_conditions: Vec<WinConditionSpec>,
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
        let mut demotes_to: HashMap<String, Option<String>> = HashMap::new();

        for piece_spec in &spec.pieces {
            demotes_to.insert(piece_spec.code.clone(), piece_spec.demotes_to.clone());
        }
        for piece_spec in spec.pieces {
            blueprints.insert(
                piece_spec.code.clone(),
                PieceBlueprint::from_spec(piece_spec.clone(), spec.players.clone()),
            );
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
                        Piece::new(piece_code.clone(), player.name.clone()),
                    );
                }
            }
        }

        // Process custom conditions.
        let mut conditions: HashMap<String, ConditionDef> = HashMap::new();
        for condition in spec.conditions {
            conditions.insert(
                condition.code.clone(),
                ConditionDef {
                    r#type: condition.r#type,
                    check: condition.check,
                },
            );
        }

        // Pre-sort each insufficient-material set so runtime comparisons are O(n log n).
        let insufficient_material = spec
            .draw_conditions
            .insufficient_material
            .into_iter()
            .map(|mut v| {
                v.sort();
                v
            })
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
            stalemate_loses: spec.stalemate_loses,
            hand_enabled: spec.hand_enabled,
            demotes_to,
            win_conditions: spec.win_conditions,
            state: GameState {
                pieces,
                current_turn,
                available_moves: None,
                phase: GamePhase::Idle,
                history: Vec::new(),
                position_hashes: Vec::new(),
                hand: HashMap::new(),
                check_counts: HashMap::new(),
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
            GameTransition::CalculateMoves { position } => self.calculate_moves(position),
            GameTransition::ExecuteMove { position } => self.execute_move(position),
            GameTransition::Transform { target } => self.transform(target),
            GameTransition::CalculateDrops { piece_code } => self.calculate_drops(piece_code),
            GameTransition::ExecuteDrop { position } => self.execute_drop(position),
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

    /// Returns the pieces in every player's hand (read-only).
    pub fn hand(&self) -> &HashMap<String, HashMap<String, u32>> {
        &self.state.hand
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
    pub(crate) fn attacked_by_pieces(
        attacker: &str,
        pieces: &HashMap<Position, Piece>,
        board: &Board,
        blueprints: &HashMap<String, PieceBlueprint>,
    ) -> HashSet<Position> {
        pieces
            .iter()
            .filter(|(_, p)| p.player == attacker)
            .filter_map(|(pos, p)| blueprints.get(&p.code).map(|bp| (pos, bp)))
            .flat_map(|(pos, bp)| bp.calculate_threats_with(attacker, pos, pieces, board))
            .collect()
    }

    /// Returns true if the current player's leader is in check given a simulated pieces map.
    /// A player is in check when *any* of their leader-coded pieces is attacked.
    fn leader_in_check_for_pieces(&self, pieces: &HashMap<Position, Piece>) -> bool {
        if self.leader.is_empty() {
            return false;
        }
        let player = self.current_player();
        let leader_positions: Vec<Position> = pieces
            .iter()
            .filter(|(_, p)| p.player == player && self.leader.contains(&p.code))
            .map(|(pos, _)| pos.clone())
            .collect();
        if leader_positions.is_empty() {
            return false;
        }
        self.players.iter().filter(|p| **p != player).any(|opp| {
            let attacks = Self::attacked_by_pieces(opp, pieces, &self.board, &self.blueprints);
            leader_positions.iter().any(|pos| attacks.contains(pos))
        })
    }

    /// Returns true if the current player's leader is currently in check.
    pub fn leader_in_check(&self) -> bool {
        self.leader_in_check_for_pieces(&self.state.pieces)
    }

    /// Returns true if the current player has at least one legal move (one that does not
    /// leave their leader in check). Short-circuits on the first legal move found.
    /// When hand_enabled, also checks whether any drop is available.
    pub fn any_legal_moves(&self) -> bool {
        let player = self.current_player();

        // Check board moves.
        let has_board_move = self
            .state
            .pieces
            .iter()
            .filter(|(_, piece)| piece.player == player)
            .any(|(pos, piece)| {
                let Some(bp) = self.blueprints.get(&piece.code) else {
                    return false;
                };
                let Some(moves) = bp.calculate_moves(piece, pos, self) else {
                    return false;
                };
                moves.values().any(|effect| {
                    let mut sim = self.state.pieces.clone();
                    for change in &effect.board_changes {
                        match &change.piece {
                            Some(p) => {
                                sim.insert(change.position.clone(), p.clone());
                            }
                            None => {
                                sim.remove(&change.position);
                            }
                        }
                    }
                    !self.leader_in_check_for_pieces(&sim)
                })
            });

        if has_board_move {
            return true;
        }

        // Check drops (only relevant when hand_enabled).
        if !self.hand_enabled {
            return false;
        }

        let piece_codes: Vec<String> = self
            .state
            .hand
            .get(&player)
            .map(|h| {
                h.iter()
                    .filter(|(_, &ref c)| *c > 0)
                    .map(|(code, _)| code.clone())
                    .collect()
            })
            .unwrap_or_default();

        for piece_code in &piece_codes {
            let restrictions = self
                .blueprints
                .get(piece_code.as_str())
                .map(|bp| bp.drop_restrictions.clone())
                .unwrap_or_default();

            for candidate in self.board.all_positions() {
                if self.state.pieces.contains_key(&candidate) {
                    continue;
                }
                let blocked = restrictions
                    .iter()
                    .any(|cond| self.check_drop_restriction(&candidate, cond, &player));
                if blocked {
                    continue;
                }
                let new_piece = Piece::new(piece_code.clone(), player.clone());
                let mut sim = self.state.pieces.clone();
                sim.insert(candidate, new_piece);
                if !self.leader_in_check_for_pieces(&sim) {
                    return true;
                }
            }
        }

        false
    }

    /// Returns true if the drop restriction `cond` fires for the candidate drop square.
    /// A restriction firing means the drop is blocked at that square.
    pub fn check_drop_restriction(
        &self,
        position: &Position,
        cond: &ConditionSpec,
        current_player: &str,
    ) -> bool {
        match cond.condition.as_str() {
            ALLY_ON_FILE => {
                let Some(ally_code) = &cond.piece else {
                    return false;
                };
                let file = position[0];
                self.state.pieces.iter().any(|(pos, p)| {
                    p.player == current_player && p.code == *ally_code && pos[0] == file
                })
            }
            // All other conditions are checked via the custom conditions map (e.g. POSITION).
            other => self.check_position_condition(position, &other.to_string()),
        }
    }

    /// Computes all legal drop squares for `piece_code` placed by `current_player`.
    pub fn compute_drop_squares(
        &self,
        piece_code: &str,
        current_player: &str,
    ) -> HashMap<Position, crate::shared::Effect> {
        use crate::shared::{BoardChange, Effect};
        let restrictions = self
            .blueprints
            .get(piece_code)
            .map(|bp| bp.drop_restrictions.clone())
            .unwrap_or_default();

        let mut available = HashMap::new();
        for candidate in self.board.all_positions() {
            if self.state.pieces.contains_key(&candidate) {
                continue;
            }
            let blocked = restrictions
                .iter()
                .any(|cond| self.check_drop_restriction(&candidate, cond, current_player));
            if blocked {
                continue;
            }
            let new_piece = Piece::new(piece_code.to_string(), current_player.to_string());
            let mut sim = self.state.pieces.clone();
            sim.insert(candidate.clone(), new_piece.clone());
            if self.leader_in_check_for_pieces(&sim) {
                continue;
            }
            available.insert(
                candidate.clone(),
                Effect {
                    action: DROP.to_string(),
                    board_changes: vec![BoardChange::set_piece(candidate, new_piece)],
                    metadata: None,
                },
            );
        }
        available
    }

    /// Builds a deterministic string key encoding the full position:
    /// active player + all pieces sorted by position, with their code, player,
    /// total_moves, and state flags. Used for repetition detection.
    fn position_key(&self) -> String {
        let mut pieces: Vec<_> = self.state.pieces.iter().collect();
        pieces.sort_by(|a, b| a.0.cmp(b.0));

        let pieces_str: String = pieces
            .iter()
            .map(|(pos, p)| {
                let pos_s = pos
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                let mut flags: Vec<_> = p.state.iter().collect();
                flags.sort_by(|a, b| a.0.cmp(b.0));
                let flags_s = flags
                    .iter()
                    .map(|(k, v)| {
                        let v_s = match v {
                            crate::logic::PieceState::Blank => "B".to_string(),
                            crate::logic::PieceState::Uint(n) => format!("U{n}"),
                            crate::logic::PieceState::String(s) => format!("S{s}"),
                        };
                        format!("{k}={v_s}")
                    })
                    .collect::<Vec<_>>()
                    .join(";");
                format!(
                    "[{pos_s}:{piece}:{player}:{mv}:{flags_s}]",
                    piece = p.code,
                    player = p.player,
                    mv = (p.total_moves > 0) as u8,
                )
            })
            .collect();

        format!("{}|{}", self.current_player(), pieces_str)
    }

    /// Records the current position and checks all configured draw conditions.
    /// Returns `true` and sets `GameOver { winner: None }` if a draw is detected.
    fn check_draws(&mut self) -> bool {
        let key = self.position_key();
        self.state.position_hashes.push(key.clone());

        // Repetition draw.
        if let Some(threshold) = self.repetition_count {
            let count = self
                .state
                .position_hashes
                .iter()
                .filter(|h| *h == &key)
                .count();
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
                    r.action != "CAPTURE" && !self.fifty_move_pawn_codes.contains(&r.piece_code)
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
                let mut player_pieces: Vec<String> = self
                    .state
                    .pieces
                    .values()
                    .filter(|p| &p.player == player)
                    .map(|p| p.code.clone())
                    .collect();
                player_pieces.sort();
                self.insufficient_material
                    .iter()
                    .any(|entry| player_pieces == *entry)
            });
            if all_insufficient {
                self.state.phase = GamePhase::GameOver { winner: None };
                return true;
            }
        }

        false
    }

    /// Determines whether the game is over after a move/transform and updates the phase.
    ///
    /// For N-player games: a checkmated player is eliminated from the turn order rather than
    /// immediately ending the game. The game ends only when one player remains.
    pub fn check_game_over(&mut self) {
        if self.check_win_conditions() {
            return;
        }
        if self.check_draws() {
            return;
        }
        if !self.any_legal_moves() {
            let is_checkmate = self.stalemate_loses || self.leader_in_check();
            if is_checkmate {
                // Checkmate (or stalemate-loses): eliminate the current player.
                let eliminated = self.current_player();
                self.turn_order.retain(|p| *p != eliminated);
                if self.turn_order.len() <= 1 {
                    let winner = self.turn_order.first().cloned();
                    self.state.phase = GamePhase::GameOver { winner };
                } else {
                    // Clamp the cursor in case it ran past the new end of the vector.
                    self.state.current_turn %= self.turn_order.len() as u8;
                    self.state.phase = GamePhase::Idle;
                }
            } else {
                // Stalemate: draw.
                self.state.phase = GamePhase::GameOver { winner: None };
            }
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
        self.check_position_condition_for_player(position, condition, &self.current_player())
    }

    /// Like `check_position_condition` but uses an explicit player instead of the current player.
    /// Used by win-condition checks that need to evaluate zones from the mover's perspective.
    pub fn check_position_condition_for_player(
        &self,
        position: &Position,
        condition: &String,
        player: &str,
    ) -> bool {
        let maybe_piece = self.piece_at_position(position);

        // Built-in occupancy conditions.
        match &maybe_piece {
            Some(p) => {
                if condition == NOT_EMPTY {
                    return true;
                }
                if condition == ALLY && p.player == player {
                    return true;
                }
                if condition == ENEMY && p.player != player {
                    return true;
                }
            }
            None => {
                if condition == EMPTY {
                    return true;
                }
            }
        };

        // Custom conditions (POSITION / STATE).
        let Some(condition_def) = self.conditions.get(condition) else {
            return false;
        };

        let condition_value = match condition_def.r#type.as_str() {
            POSITION => &into_string(position),
            STATE => condition,
            _ => return false,
        };

        let Some(player_set) = condition_def.check.get(player) else {
            return false;
        };
        player_set.contains(condition_value)
    }

    /// Evaluates all configured win conditions after a move.
    /// Returns `true` and sets `GamePhase::GameOver` if a win is detected.
    fn check_win_conditions(&mut self) -> bool {
        let prev_player = self.previous_player();
        let wcs = self.win_conditions.clone();

        for wc in &wcs {
            match wc.r#type.as_str() {
                PIECE_IN_ZONE => {
                    let (Some(piece_code), Some(zone)) = (&wc.piece, &wc.zone) else {
                        continue;
                    };
                    let positions: Vec<Position> = self
                        .state
                        .pieces
                        .iter()
                        .filter(|(_, p)| p.player == prev_player && &p.code == piece_code)
                        .map(|(pos, _)| pos.clone())
                        .collect();
                    let won = positions.iter().any(|pos| {
                        self.check_position_condition_for_player(pos, zone, &prev_player)
                    });
                    if won {
                        self.state.phase = GamePhase::GameOver {
                            winner: Some(prev_player),
                        };
                        return true;
                    }
                }

                OPPONENT_BARE => {
                    let won = self
                        .players
                        .iter()
                        .filter(|p| p.as_str() != prev_player.as_str())
                        .all(|opp| {
                            self.state
                                .pieces
                                .values()
                                .filter(|p| p.player == *opp)
                                .all(|p| wc.exempt.contains(&p.code))
                        });
                    if won {
                        self.state.phase = GamePhase::GameOver {
                            winner: Some(prev_player),
                        };
                        return true;
                    }
                }

                CHECK_COUNT => {
                    let threshold = wc.threshold.unwrap_or(3);
                    // If the current player (who is now about to move) is in check,
                    // the previous player (who just moved) delivered that check.
                    if self.leader_in_check() {
                        let count = self
                            .state
                            .check_counts
                            .entry(prev_player.clone())
                            .or_insert(0);
                        *count += 1;
                        if *count >= threshold {
                            self.state.phase = GamePhase::GameOver {
                                winner: Some(prev_player),
                            };
                            return true;
                        }
                    }
                }

                _ => {}
            }
        }

        false
    }
}
