use std::collections::{HashMap, HashSet};

use crate::shared::Position;
use crate::specs::{GameSpec, WinConditionSpec};

use super::{Board, ConditionDef, GameError, GamePhase, GameState, GameTransition, Piece};
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

    /// Piece code → what it is worth in the material count. Pieces without a value are left out.
    pub piece_values: HashMap<String, u32>,

    /// Maps piece code → the code that enters the hand on capture.
    /// `None` means the piece itself (base form) enters the hand.
    pub demotes_to: HashMap<String, Option<String>>,

    /// Win conditions checked after every move, before draw/checkmate detection.
    pub win_conditions: Vec<WinConditionSpec>,

    /// Players whose `starting_positions` included at least one leader-coded piece.
    /// Used to distinguish "leader was destroyed" from "leader was never placed" (test setups).
    pub players_with_configured_leader: HashSet<String>,

    /// When `true`, every generated move is post-filtered to ensure it does not put any
    /// opponent's leader in check. Used in Racing Kings where checking the opponent is illegal.
    pub no_opponent_check: bool,

    /// When `true`, a piece that captures may continue jumping (draughts multi-jump).
    pub jump_enabled: bool,

    /// When `true`, captures are mandatory if any are available anywhere on the board.
    pub mandatory_capture: bool,
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

        let mut piece_values: HashMap<String, u32> = HashMap::new();

        for piece_spec in &spec.pieces {
            demotes_to.insert(piece_spec.code.clone(), piece_spec.demotes_to.clone());
            if let Some(value) = piece_spec.value {
                piece_values.insert(piece_spec.code.clone(), value);
            }
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
        let mut players_with_configured_leader: HashSet<String> = HashSet::new();

        for player in spec.players.into_iter() {
            // Store players' names (identifiers).
            players.push(player.name.clone());

            // Add pieces to the board, based on the starting positions for each player.
            for starting_positions in player.starting_positions {
                let piece_code = starting_positions.piece;

                // Track whether any player's starting configuration includes a leader piece.
                if spec.leader.contains(&piece_code) {
                    players_with_configured_leader.insert(player.name.clone());
                }

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
            players_with_configured_leader,
            repetition_count: spec.draw_conditions.repetition_count,
            fifty_move_halfmoves: spec.draw_conditions.fifty_move_halfmoves,
            fifty_move_pawn_codes: spec.draw_conditions.fifty_move_pawn_codes,
            insufficient_material,
            stalemate_loses: spec.stalemate_loses,
            hand_enabled: spec.hand_enabled,
            no_opponent_check: spec.no_opponent_check,
            jump_enabled: spec.jump_enabled,
            mandatory_capture: spec.mandatory_capture,
            demotes_to,
            piece_values,
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
                captured: HashMap::new(),
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

    pub(crate) fn current_player_ref(&self) -> &str {
        self.turn_order
            .get(self.state.current_turn as usize)
            .or_else(|| self.turn_order.first())
            .map_or("", String::as_str)
    }

    pub fn current_player(&self) -> String {
        self.current_player_ref().to_string()
    }

    pub(crate) fn piece_at_position_ref(&self, position: &Position) -> Option<&Piece> {
        self.state.pieces.get(position)
    }

    /// Finds the piece at a given position. If no piece is present, return None.
    pub fn piece_at_position(&self, position: &Position) -> Option<Piece> {
        self.piece_at_position_ref(position).cloned()
    }

    /// Returns the pieces in every player's hand (read-only).
    pub fn hand(&self) -> &HashMap<String, HashMap<String, u32>> {
        &self.state.hand
    }
}
