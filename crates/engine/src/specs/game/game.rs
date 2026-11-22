use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io;
use thiserror::Error;

use crate::shared::{Direction, ADJACENT, CONNECTION, ORTHOGONAL, POSITION};
use crate::specs::Validate;

use super::board::{BoardSpec, PlayerSpec, TurnSpec, MAX_STEP};
use super::condition::ConditionSpec;
use super::draw_conditions::DrawConditionsSpec;
use super::liberty_capture::LibertyCaptureSpec;
use super::piece::side_effect::SideEffectSpec;
use super::piece::{ConditionSpec as PieceConditionSpec, PieceSpec};
use super::win_condition::WinConditionSpec;

/// Full spec of a game, to be read from a .json file.
#[derive(Debug, Deserialize, Serialize)]
pub struct GameSpec {
    /// A game must have a name, such as "chess".
    pub name: String,

    /// A list of piece names known to the game. The specs for each of them must be loaded into the game.
    #[serde(default = "default_pieces")]
    pub pieces: Vec<PieceSpec>,

    /// A prescription of the board. It has its own spec.
    pub board: BoardSpec,

    /// Player specs, determining player-specific values - such as player names (i.e. WHITE).
    pub players: Vec<PlayerSpec>,

    /// Turn spec - including order of turns, and starting position (in turn order vector).
    pub turns: TurnSpec,

    /// Global custom conditions that can be referenced by pieces.
    #[serde(default = "default_conditions")]
    pub conditions: Vec<ConditionSpec>,

    /// The piece codes that act as leaders (must not be in check).
    /// A player is in check if *any* of their leader pieces is attacked.
    /// Empty list disables checkmate/stalemate detection.
    #[serde(default)]
    pub leader: Vec<String>,

    /// Optional draw-condition rules (50-move, repetition, insufficient material).
    /// All sub-fields default to disabled, so games that omit this field are unaffected.
    #[serde(default)]
    pub draw_conditions: DrawConditionsSpec,

    /// When `true`, a player with no legal moves always loses (Shogi rule).
    /// When `false` (the default), no legal moves without check is a stalemate draw.
    #[serde(default)]
    pub stalemate_loses: bool,

    /// When `true`, captured pieces go into the capturing player's hand and can be
    /// dropped back onto the board. Disabled by default so existing games are unaffected.
    #[serde(default)]
    pub hand_enabled: bool,

    /// Win conditions checked after every move. When one fires the moving player wins
    /// immediately. Evaluated before draw conditions and checkmate detection.
    #[serde(default)]
    pub win_conditions: Vec<WinConditionSpec>,

    /// When `true`, every generated move is post-filtered to ensure it does not leave any
    /// opponent's leader in check. Used in Racing Kings where checking the opponent is illegal.
    #[serde(default)]
    pub no_opponent_check: bool,

    /// When `true`, a piece that captures may continue jumping over additional enemy pieces
    /// (draughts-style multi-jump). The turn does not advance until no further captures exist.
    #[serde(default)]
    pub jump_enabled: bool,

    /// When `true`, if any current-player piece has at least one capture available, all
    /// non-capture moves are filtered out (draughts mandatory-capture rule).
    #[serde(default)]
    pub mandatory_capture: bool,

    /// When `true`, the player to move may give up their turn instead of moving. A pass is
    /// recorded in the history with the action `PASS`. Defaults to `false`.
    #[serde(default)]
    pub allow_pass: bool,

    /// When `true`, a move or placement may not leave the position (pieces and player to move)
    /// as it was after any earlier turn. This is the ko rule in Go. Defaults to `false`; for a
    /// draw on repetition instead, see `draw_conditions.repetition_count`.
    #[serde(default)]
    pub forbid_repeated_positions: bool,

    /// Captures groups that have no empty point next to them. Absent means no such captures.
    #[serde(default)]
    pub liberty_capture: Option<LibertyCaptureSpec>,
}

fn default_pieces() -> Vec<PieceSpec> {
    vec![]
}

fn default_conditions() -> Vec<ConditionSpec> {
    vec![]
}

#[derive(Error, Debug)]
pub enum GameSpecError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// A player name is repeated in the spec.
    #[error("Duplicate player name error: {0}")]
    DuplicatePlayerName(String),

    /// A player in the turn order spec is not known.
    #[error("Unknown player name in turn order: {0}")]
    UnknownPlayerInTurnOrder(String),

    /// Some piece name in the starting positions is unknown.
    #[error("Unknown piece name in starting positions: {0}")]
    UnknownPieceInStartingPosition(String),

    /// Some position has dimensions different than the board.
    #[error("Position has invalid dimensions: {0:?}")]
    InvalidPositionDimensions(Vec<u8>),

    /// The board has no dimensions, a dimension of size 0, or too many dimensions or positions.
    #[error("The board's dimensions are empty, zero or too large: {0:?}")]
    InvalidBoardSize(Vec<u8>),

    /// A direction matrix, step or relative position does not have one entry per dimension of
    /// the board.
    #[error("The {what} has {found} entries, but the board has {expected} dimensions")]
    MismatchedDimensions {
        what: String,
        expected: usize,
        found: usize,
    },

    /// A step or relative position reaches further than any board is wide.
    #[error("The {what} has the value {value}, further than any board reaches")]
    ValueOutOfRange { what: String, value: i16 },

    /// The direction matrix is not a valid rotation (determinant must be ±1).
    #[error("Direction is not a valid rotation matrix (det must be ±1): {0:?}")]
    InvalidDirectionMatrix(Direction),

    /// A specified position has been marked as disabled on the board.
    #[error("The specified position is disabled on the board: {0:?}")]
    PositionDisabled(Vec<u8>),

    /// A move whose step has no distance, so it would never leave its square.
    #[error("Move {1} of piece {0} has a step that goes nowhere")]
    MoveGoesNowhere(String, u8),

    /// The turn order names nobody, so nobody could ever move.
    #[error("The turn order must name at least one player")]
    EmptyTurnOrder,

    /// A rule (liberty capture, area score, win condition) names a piece the game does not have.
    #[error("Unknown piece name in a rule: {0}")]
    UnknownPieceInRule(String),

    /// A win condition is missing a setting it needs, or names something the game does not have.
    #[error("Invalid win condition: {0}")]
    InvalidWinCondition(String),
}

impl Validate for GameSpec {
    type Arg1 = ();
    type Arg2 = ();

    /// Validates the consistency of the game spec.
    fn validate(&self, _1: &(), _2: &()) -> Result<(), GameSpecError> {
        self.board.validate_size()?;

        let player_names: HashSet<String> = self.players.iter().map(|p| p.name.clone()).collect();
        let piece_names: HashSet<String> = self.pieces.iter().map(|p| p.code.clone()).collect();

        self.validate_player_specs(&piece_names)?;
        self.validate_player_name_duplicates(&player_names)?;
        self.validate_players_in_turn_order(&player_names)?;
        self.validate_rule_pieces(&piece_names)?;
        self.validate_connections()?;
        self.validate_steps()?;
        self.validate_vector_sizes()?;

        Ok(())
    }
}

impl GameSpec {
    pub fn validate_specs(&self) -> Result<(), GameSpecError> {
        self.validate(&(), &())
    }

    /// Validates whether if player names are repeated or not.
    fn validate_player_name_duplicates(
        &self,
        player_names: &HashSet<String>,
    ) -> Result<(), GameSpecError> {
        // Check for duplicate player names
        if player_names.len() != self.players.len() {
            for player in &self.players {
                if self
                    .players
                    .iter()
                    .filter(|p| p.name == player.name)
                    .count()
                    > 1
                {
                    return Err(GameSpecError::DuplicatePlayerName(player.name.clone()));
                }
            }
        }

        Ok(())
    }

    /// Validates that all the values specified in turn order are valid player names.
    fn validate_players_in_turn_order(
        &self,
        player_names: &HashSet<String>,
    ) -> Result<(), GameSpecError> {
        if self.turns.order.is_empty() {
            return Err(GameSpecError::EmptyTurnOrder);
        }
        // Check that all players in turn order exist in player specs
        for player_name in &self.turns.order {
            if !player_names.contains(player_name) {
                return Err(GameSpecError::UnknownPlayerInTurnOrder(player_name.clone()));
            }
        }

        Ok(())
    }

    /// Validates that every move's step covers some distance. A repeating move with no
    /// distance stays on its square forever.
    fn validate_steps(&self) -> Result<(), GameSpecError> {
        for piece in &self.pieces {
            for movement in &piece.moves {
                if movement.step.iter().all(|distance| *distance == 0) {
                    return Err(GameSpecError::MoveGoesNowhere(
                        piece.code.clone(),
                        movement.id,
                    ));
                }
            }
        }
        Ok(())
    }

    /// Validates that every move step and every relative position in a condition or side effect
    /// has one entry per dimension of the board, none longer than `MAX_STEP`.
    fn validate_vector_sizes(&self) -> Result<(), GameSpecError> {
        let expected = self.board.dimensions.len();
        let check = |what: String, vector: &[i16]| {
            if vector.len() != expected {
                return Err(GameSpecError::MismatchedDimensions {
                    what,
                    expected,
                    found: vector.len(),
                });
            }
            match vector
                .iter()
                .find(|value| value.unsigned_abs() > MAX_STEP.unsigned_abs())
            {
                Some(&value) => Err(GameSpecError::ValueOutOfRange { what, value }),
                None => Ok(()),
            }
        };
        let check_condition =
            |where_: &str, condition: &PieceConditionSpec| match &condition.position {
                Some(position) => check(format!("position of a condition in {where_}"), position),
                None => Ok(()),
            };
        let check_side_effect = |where_: &str, side_effect: &SideEffectSpec| {
            for (name, vector) in [
                ("from", &side_effect.from),
                ("to", &side_effect.to),
                ("target", &side_effect.target),
            ] {
                if let Some(vector) = vector {
                    check(format!("{name} of a side effect in {where_}"), vector)?;
                }
            }
            match &side_effect.condition {
                Some(condition) => check_condition(where_, condition),
                None => Ok(()),
            }
        };

        for piece in &self.pieces {
            let piece_where = format!("piece {}", piece.code);
            for condition in &piece.drop_restrictions {
                check_condition(&piece_where, condition)?;
            }
            for movement in &piece.moves {
                let where_ = format!("move {} of piece {}", movement.id, piece.code);
                check(format!("step of {where_}"), &movement.step)?;
                for condition in &movement.conditions {
                    check_condition(&where_, condition)?;
                }
                for side_effect in &movement.side_effects {
                    check_side_effect(&where_, side_effect)?;
                }
                for modifier in &movement.modifiers {
                    for condition in &modifier.conditions {
                        check_condition(&where_, condition)?;
                    }
                }
                for action in &movement.actions {
                    for condition in &action.conditions {
                        check_condition(&where_, condition)?;
                    }
                    for side_effect in &action.side_effects {
                        check_side_effect(&where_, side_effect)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Validates that the piece codes named by the liberty and area-score rules exist.
    fn validate_rule_pieces(&self, piece_names: &HashSet<String>) -> Result<(), GameSpecError> {
        let from_liberties = self
            .liberty_capture
            .iter()
            .flat_map(|rule| rule.pieces.iter());
        let from_scoring = self.win_conditions.iter().flat_map(|wc| wc.pieces.iter());
        for code in from_liberties.chain(from_scoring) {
            if !piece_names.contains(code) {
                return Err(GameSpecError::UnknownPieceInRule(code.clone()));
            }
        }
        Ok(())
    }

    /// A CONNECTION win condition needs a known neighbourhood, zones that are declared
    /// POSITION conditions, and an `at_least` between 1 and the number of zones.
    fn validate_connections(&self) -> Result<(), GameSpecError> {
        let refused = |why: String| Err(GameSpecError::InvalidWinCondition(why));
        for rule in self
            .win_conditions
            .iter()
            .filter(|wc| wc.r#type == CONNECTION)
        {
            match rule.through.as_deref() {
                Some(ORTHOGONAL) | Some(ADJACENT) => {}
                other => return refused(format!("unknown neighbourhood {other:?}")),
            }
            if rule.zones.is_empty() {
                return refused("a connection needs at least one zone".to_string());
            }
            for zone in &rule.zones {
                let declared = self
                    .conditions
                    .iter()
                    .any(|c| &c.code == zone && c.r#type == POSITION);
                if !declared {
                    return refused(format!("unknown zone {zone}"));
                }
            }
            if let Some(at_least) = rule.at_least {
                if at_least == 0 || usize::from(at_least) > rule.zones.len() {
                    return refused(format!(
                        "at_least is {at_least}, but there are {} zones",
                        rule.zones.len()
                    ));
                }
            }
        }
        Ok(())
    }

    /// Validates players specs to be valid.
    fn validate_player_specs(&self, piece_names: &HashSet<String>) -> Result<(), GameSpecError> {
        for player in &self.players {
            player.validate(&piece_names, &self.board)?;
        }

        Ok(())
    }
}
