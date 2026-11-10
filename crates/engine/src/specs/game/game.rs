use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io;
use thiserror::Error;

use crate::shared::Direction;
use crate::specs::Validate;

use super::board::{BoardSpec, PlayerSpec, TurnSpec};
use super::condition::ConditionSpec;
use super::draw_conditions::DrawConditionsSpec;
use super::liberty_capture::LibertyCaptureSpec;
use super::piece::PieceSpec;
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

    /// The direction matrix is not a valid rotation (determinant must be ±1).
    #[error("Direction is not a valid rotation matrix (det must be ±1): {0:?}")]
    InvalidDirectionMatrix(Direction),

    /// A specified position has been marked as disabled on the board.
    #[error("The specified position is disabled on the board: {0:?}")]
    PositionDisabled(Vec<u8>),

    /// A rule (liberty capture, area score, win condition) names a piece the game does not have.
    #[error("Unknown piece name in a rule: {0}")]
    UnknownPieceInRule(String),
}

impl Validate for GameSpec {
    type Arg1 = ();
    type Arg2 = ();

    /// Validates the consistency of the game spec.
    fn validate(&self, _1: &(), _2: &()) -> Result<(), GameSpecError> {
        let player_names: HashSet<String> = self.players.iter().map(|p| p.name.clone()).collect();
        let piece_names: HashSet<String> = self.pieces.iter().map(|p| p.code.clone()).collect();

        self.validate_player_specs(&piece_names)?;
        self.validate_player_name_duplicates(&player_names)?;
        self.validate_players_in_turn_order(&player_names)?;
        self.validate_rule_pieces(&piece_names)?;

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
        // Check that all players in turn order exist in player specs
        for player_name in &self.turns.order {
            if !player_names.contains(player_name) {
                return Err(GameSpecError::UnknownPlayerInTurnOrder(player_name.clone()));
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

    /// Validates players specs to be valid.
    fn validate_player_specs(&self, piece_names: &HashSet<String>) -> Result<(), GameSpecError> {
        for player in &self.players {
            player.validate(&piece_names, &self.board)?;
        }

        Ok(())
    }
}
