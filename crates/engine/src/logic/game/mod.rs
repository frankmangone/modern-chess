pub(crate) mod attack;
pub mod board;
pub(crate) mod board_changes;
pub(crate) mod connection;
pub(crate) mod draw;
pub mod error;
pub(crate) mod handlers;
pub(crate) mod io;
pub(crate) mod legal;
pub(crate) mod liberties;
pub mod phase;
pub(crate) mod scoring;
pub mod state;
pub mod transition;
pub(crate) mod transitions;
pub(crate) mod ways;
pub mod win;
pub mod zones;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::shared::Position;
use crate::specs::{GameSpec, LibertyCaptureSpec};

use crate::logic::blueprint::steps::{self, Action};
use crate::logic::blueprint::test_block::Test;
use crate::logic::blueprint::PieceBlueprint;
use board::{graph::Labels, Board};
use error::GameError;
use phase::GamePhase;
use state::{piece::Piece, GameState};
use transition::GameTransition;
use win::WinCondition;
use zones::Zone;

/// What a game is built with and never changes while it is played: shared by every copy of
/// the game, so trying a position copies only the state.
#[derive(Debug, Clone)]
pub struct Rules {
    /// The order in which players play, as the spec gives it. The order being played is
    /// `state.turn_order`, which loses players as they are eliminated.
    pub turn_order: Vec<String>,

    // Name of the game, for identification purposes only.
    pub name: String,

    /// The zones declared under the spec's `zones`, by name.
    pub zones: HashMap<String, Zone>,

    // A list of available players. Doubles up as a sort of dynamic enum.
    pub players: Vec<String>,

    // `blueprints` allow for calculation of piece movements.
    pub blueprints: HashMap<String, PieceBlueprint>,

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
    pub win_conditions: Vec<WinCondition>,

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

    /// When `true`, a move may not recreate a position that has already occurred.
    pub forbid_repeated_positions: bool,

    /// Capture of groups that have no liberties, if the game has it.
    pub liberty_capture: Option<LibertyCaptureSpec>,

    /// What a player may do on their turn, in spec order.
    pub actions: Vec<Action>,

    /// Tests that narrow the ways, in order (see `GameSpec::must`).
    pub must: Vec<Test>,

    /// Whether some move or action can remove more than its target (a `REMOVE` inside a
    /// `FOR_EACH`, or at `START`), so that check detection must simulate a capture of a leader.
    pub removes_beyond_target: bool,
}

#[derive(Debug, Clone)]
pub struct Game {
    /// The rules the game was built with.
    pub rules: Arc<Rules>,

    // The `board` stores info about dimensions and disabled positions.
    pub board: Board,

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
        // The blueprints collect every edge label their rules follow; the board graph is then
        // compiled with an edge table for each.
        let mut labels = Labels::default();
        for piece_spec in spec.pieces {
            blueprints.insert(
                piece_spec.code.clone(),
                PieceBlueprint::from_spec(piece_spec.clone(), spec.players.clone(), &mut labels),
            );
        }
        let actions: Vec<Action> = spec
            .actions
            .iter()
            .map(|action| Action::build(action, &spec.every_action, &spec.players, &mut labels))
            .collect();
        let must: Vec<Test> = spec
            .must
            .iter()
            .map(|test| Test::build(test, &spec.players, None, &mut labels))
            .collect();
        let move_steps = blueprints
            .values()
            .flat_map(|blueprint: &PieceBlueprint| &blueprint.move_blueprints)
            .map(|movement| &movement.steps);
        let removes_beyond_target = actions
            .iter()
            .map(|action| &action.steps)
            .chain(move_steps)
            .flat_map(|by_player| by_player.values())
            .any(|steps| steps::removes_beyond_target(steps));
        let board = Board::new(spec.board, &labels);

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
                        position.position(),
                        Piece::new(piece_code.clone(), player.name.clone()),
                    );
                }
            }
        }

        let zones: HashMap<String, Zone> = spec
            .zones
            .into_iter()
            .map(|zone| {
                let places = zone
                    .places
                    .into_iter()
                    .map(|(player, places)| {
                        let names = places.iter().map(|p| p.position().to_string()).collect();
                        (player, names)
                    })
                    .collect();
                (zone.name, Zone { places })
            })
            .collect();

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

        let mut game = Game {
            rules: Arc::new(Rules {
                turn_order: turn_order.clone(),
                name: spec.name,
                zones,
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
                forbid_repeated_positions: spec.forbid_repeated_positions,
                liberty_capture: spec.liberty_capture,
                actions,
                must,
                removes_beyond_target,
                demotes_to,
                piece_values,
                win_conditions: spec
                    .win_conditions
                    .into_iter()
                    .map(WinCondition::from)
                    .collect(),
                blueprints,
            }),
            board,
            state: GameState {
                turn_order,
                pieces,
                current_turn,
                ways: None,
                phase: GamePhase::Choosing { chosen: Vec::new() },
                continuing: None,
                history: Vec::new(),
                position_hashes: Vec::new(),
                hand: HashMap::new(),
                check_counts: HashMap::new(),
                captured: HashMap::new(),
            },
        };
        game.work_out_ways();
        game
    }

    // ---------------------------------------------------------------------
    // Main transition function
    // ---------------------------------------------------------------------
    pub fn transition(&mut self, transition: GameTransition) -> Result<(), GameError> {
        match transition {
            GameTransition::Choose(answer) => self.choose(answer),
            GameTransition::Cancel => self.cancel(),
        }
    }

    // ---------------------------------------------------------------------
    // Utility functions
    // ---------------------------------------------------------------------
    pub fn next_turn(&mut self) {
        let new_turn = self.state.current_turn + 1;

        if new_turn >= self.state.turn_order.len() as u8 {
            self.state.current_turn = 0;
        } else {
            self.state.current_turn = new_turn;
        }

        // Tick duration-tracked state flags on every piece.
        for piece in self.state.pieces.values_mut() {
            piece.tick_state_flags();
        }
    }

    /// The player whose turn it will be once the current player has finished.
    pub(crate) fn player_after_current(&self) -> &str {
        let next = (self.state.current_turn as usize + 1) % self.state.turn_order.len().max(1);
        self.state
            .turn_order
            .get(next)
            .or_else(|| self.state.turn_order.first())
            .map_or("", String::as_str)
    }

    pub(crate) fn current_player_ref(&self) -> &str {
        self.state
            .turn_order
            .get(self.state.current_turn as usize)
            .or_else(|| self.state.turn_order.first())
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
