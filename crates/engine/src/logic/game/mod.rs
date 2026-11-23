pub(crate) mod attack;
pub mod board;
pub(crate) mod board_changes;
pub(crate) mod draw;
pub(crate) mod ends;
pub mod error;
pub(crate) mod handlers;
pub(crate) mod io;
pub(crate) mod legal;
pub mod phase;
pub mod state;
pub mod transition;
pub(crate) mod transitions;
pub(crate) mod ways;
pub(crate) mod win;
pub mod zones;

use crate::shared::IdMap;

use std::collections::HashMap;
use std::sync::Arc;

use crate::shared::{Position, Symbol};
use crate::specs::GameSpec;

use crate::logic::blueprint::steps::{self, Action};
use crate::logic::blueprint::test_block::Test;
use crate::logic::blueprint::PieceBlueprint;
use board::{graph::Labels, Board};
use error::GameError;
use phase::GamePhase;
use state::{piece::Piece, GameState};
use transition::GameTransition;
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
    pub blueprints: IdMap<Symbol, PieceBlueprint>,

    /// When `true`, captured pieces enter the capturer's hand and can be dropped.
    pub hand_enabled: bool,

    /// Piece code → what it is worth in the material count. Pieces without a value are left out.
    pub piece_values: HashMap<String, u32>,

    /// Maps piece code → the code that enters a hand when the piece is sent there.
    /// `None` means the piece itself (base form) enters the hand.
    pub captured_as: IdMap<Symbol, Option<Symbol>>,

    /// When `true`, a piece that captures may continue jumping (draughts multi-jump).
    pub jump_enabled: bool,

    /// What a player may do on their turn, in spec order.
    pub actions: Vec<Action>,

    /// Whether some test or filter of the spec reads `moved` (castling, a pawn's double step),
    /// so that the position record must tell a piece that has moved from one that has not.
    pub moved_matters: bool,

    /// The end checks, judged in order after every turn (see `GameSpec::end_checks`).
    pub end_checks: Vec<ends::EndCheck>,

    /// The terms of the score, summed for each player (see `GameSpec::score`).
    pub score: Vec<ends::ScoreTerm>,

    /// Tests that narrow the ways, in order (see `GameSpec::must`).
    pub must: Vec<Test>,

    /// Whether some move or action can remove more than its target (a `REMOVE` inside a
    /// `FOR_EACH`, or at `START`), so that an `ATTACKED` test with `keeping` must simulate the capture.
    pub removes_beyond_target: bool,
}

/// Whether any object inside `value` has a `moved` key holding a bool: a filter that asks
/// whether a piece has moved.
fn reads_moved(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            map.get("moved").is_some_and(serde_json::Value::is_boolean)
                || map.values().any(reads_moved)
        }
        serde_json::Value::Array(items) => items.iter().any(reads_moved),
        _ => false,
    }
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
        let moved_matters = serde_json::to_value(&spec)
            .map(|value| reads_moved(&value))
            .unwrap_or(true);
        // Process turn information.
        let turn_order = spec.turns.order;
        let current_turn = spec.turns.start_at;

        // Create blueprints for each piece & player.
        // TODO: Optimize for pieces that are not direction-dependent.
        let mut blueprints: IdMap<Symbol, PieceBlueprint> = IdMap::default();
        let mut captured_as: IdMap<Symbol, Option<Symbol>> = IdMap::default();

        let mut piece_values: HashMap<String, u32> = HashMap::new();

        for piece_spec in &spec.pieces {
            captured_as.insert(
                Symbol::new(&piece_spec.code),
                piece_spec.captured_as.as_deref().map(Symbol::new),
            );
            if let Some(value) = piece_spec.value {
                piece_values.insert(piece_spec.code.clone(), value);
            }
        }
        // The blueprints collect every edge label their rules follow; the board graph is then
        // compiled with an edge table for each.
        let mut labels = Labels::default();
        for piece_spec in spec.pieces {
            blueprints.insert(
                Symbol::new(&piece_spec.code),
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
        let end_checks: Vec<ends::EndCheck> = spec
            .end_checks
            .iter()
            .map(|check| ends::EndCheck::build(check, &spec.players, &mut labels))
            .collect();
        let score: Vec<ends::ScoreTerm> = spec.score.iter().map(ends::ScoreTerm::from).collect();
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
        let mut pieces: IdMap<Position, Piece> = IdMap::default();

        for player in spec.players.into_iter() {
            // Store players' names (identifiers).
            players.push(player.name.clone());

            // Add pieces to the board, based on the starting positions for each player.
            for starting_positions in player.starting_positions {
                let piece_code = starting_positions.piece;

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

        let mut game = Game {
            rules: Arc::new(Rules {
                turn_order: turn_order.clone(),
                name: spec.name,
                zones,
                players,
                hand_enabled: spec.hand.enabled,
                moved_matters,
                jump_enabled: spec.jump_enabled,
                actions,
                must,
                end_checks,
                score,
                removes_beyond_target,
                captured_as,
                piece_values,
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
                counters: HashMap::new(),
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

    /// The value of `player`'s counter `name`; 0 when it was never changed.
    pub fn counter(&self, player: &str, name: &str) -> i64 {
        self.state
            .counters
            .get(player)
            .and_then(|counters| counters.get(name))
            .copied()
            .unwrap_or(0)
    }

    /// Returns the pieces in every player's hand (read-only).
    pub fn hand(&self) -> &HashMap<String, HashMap<String, u32>> {
        &self.state.hand
    }
}
