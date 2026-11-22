use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io;
use thiserror::Error;

use crate::shared::{
    Direction, ADJACENT, ALLY, ALLY_ON_FILE, CONNECTION, ENEMY, ORTHOGONAL, POSITION,
};
use crate::specs::Validate;

use super::board::{BoardSpec, PlayerSpec, TurnSpec, MAX_STEP};
use super::condition::{ConditionSpec, ZoneSpec};
use super::draw_conditions::DrawConditionsSpec;
use super::liberty_capture::LibertyCaptureSpec;
use super::map::MAP_LABELS;
use super::piece::side_effect::SideEffectSpec;
use super::piece::{ConditionSpec as PieceConditionSpec, PieceSpec, StepSpec, UntilSpec};
use super::player::{is_enabled_square, PositionSpec};
use super::test_block::{
    ConditionEntry, FilterSpec, PlaceSpec, RegionSpec, TestSpec, ANY, FROM, START, TARGET, TO,
};
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

    /// Named zones: for each player, the places of their part. Read by `IN_ZONE` tests, piece
    /// restrictions, modifiers and win conditions, like a `POSITION` condition.
    #[serde(default)]
    pub zones: Vec<ZoneSpec>,

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

    /// A map's places or links are not valid; the message names the place at fault.
    #[error("Invalid map: {0}")]
    InvalidMap(String),

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

    /// A starting position or zone names no place of the map, or no enabled square of the grid.
    #[error("Unknown position: no place or square is called `{0}`")]
    UnknownPosition(String),

    /// A rule written for grids (a vector, a direction, a file) on a map; the message says which.
    #[error("Not possible on a map: {0}")]
    NotOnAMap(String),

    /// A rule written for maps (a label step) on a grid; the message says which.
    #[error("Not possible on a grid: {0}")]
    NotOnAGrid(String),

    /// A label step names a label that maps do not have.
    #[error(
        "The {what} names the label `{label}`; a map's labels are next, ends, paths, corners, \
         areas and sides"
    )]
    UnknownLabel { what: String, label: String },

    /// A test names something tests do not have; the message says what and where.
    #[error("Invalid test: {0}")]
    InvalidTest(String),

    /// A zone name is declared twice, under `zones` or beside a condition of `conditions`.
    #[error("Duplicate zone name: {0}")]
    DuplicateZone(String),
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
        self.validate_zone_names()?;
        self.validate_connections()?;
        self.validate_label_steps()?;
        self.validate_map_positions()?;
        self.validate_steps()?;
        self.validate_vector_sizes()?;
        self.validate_tests(&piece_names)?;

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

    /// A label step belongs to a map, names one of its labels and does not repeat (a label
    /// can lead to several places, so a repeat has no single meaning).
    fn validate_label_steps(&self) -> Result<(), GameSpecError> {
        let on_map = self.board.map.is_some();
        for piece in &self.pieces {
            for movement in &piece.moves {
                let StepSpec::Label(label) = &movement.step else {
                    continue;
                };
                let what = format!("step of move {} of piece {}", movement.id, piece.code);
                if !on_map {
                    return Err(GameSpecError::NotOnAGrid(format!(
                        "the {what} is the label `{label}`; a grid's steps are vectors such as \
                         [0, 1]"
                    )));
                }
                if !MAP_LABELS.contains(&label.as_str()) {
                    return Err(GameSpecError::UnknownLabel {
                        what,
                        label: label.clone(),
                    });
                }
                if movement.repeat.is_some() {
                    return Err(GameSpecError::NotOnAMap(format!(
                        "move {} of piece {} has a `repeat`; a step along a label can lead to \
                         several places, so it cannot repeat",
                        movement.id, piece.code
                    )));
                }
            }
        }
        Ok(())
    }

    /// On a grid, a place of a zone or of a POSITION condition written as a name must name an
    /// enabled square. On a map, every such place is the name of a place, and no piece is
    /// restricted by its file (`ALLY_ON_FILE`), which a map does not have.
    fn validate_map_positions(&self) -> Result<(), GameSpecError> {
        let Some(map) = self.board.map.as_deref() else {
            for condition in self.conditions.iter().filter(|c| c.r#type == POSITION) {
                if let Some(name) = condition
                    .written_names
                    .iter()
                    .find(|name| !is_enabled_square(name, &self.board))
                {
                    return Err(GameSpecError::UnknownPosition(name.clone()));
                }
            }
            for zone in &self.zones {
                if let Some(name) = written_names(zone)
                    .into_iter()
                    .find(|name| !is_enabled_square(name, &self.board))
                {
                    return Err(GameSpecError::UnknownPosition(name.clone()));
                }
            }
            return Ok(());
        };
        let places: HashSet<&str> = map.places.iter().map(|p| p.name.as_str()).collect();
        for condition in self.conditions.iter().filter(|c| c.r#type == POSITION) {
            if let Some(coordinates) = &condition.written_coordinates {
                return Err(GameSpecError::NotOnAMap(format!(
                    "the condition {} names the coordinates {coordinates:?}; a map's positions \
                     are place names",
                    condition.code
                )));
            }
            let mut names: Vec<&String> = condition.check.values().flatten().collect();
            names.sort();
            if let Some(name) = names
                .into_iter()
                .find(|name| !places.contains(name.as_str()))
            {
                return Err(GameSpecError::UnknownPosition(name.clone()));
            }
        }
        for zone in &self.zones {
            let coordinates = zone
                .places
                .values()
                .flatten()
                .find_map(|place| match place {
                    PositionSpec::Coordinates(coordinates) => Some(coordinates),
                    PositionSpec::Name(_) => None,
                });
            if let Some(coordinates) = coordinates {
                return Err(GameSpecError::NotOnAMap(format!(
                    "the zone {} names the coordinates {coordinates:?}; a map's positions are \
                     place names",
                    zone.name
                )));
            }
            if let Some(name) = written_names(zone)
                .into_iter()
                .find(|name| !places.contains(name.as_str()))
            {
                return Err(GameSpecError::UnknownPosition(name.clone()));
            }
        }
        for piece in &self.pieces {
            if piece
                .drop_restrictions
                .iter()
                .any(|restriction| restriction.condition == ALLY_ON_FILE)
            {
                return Err(GameSpecError::NotOnAMap(format!(
                    "piece {} has the drop restriction {ALLY_ON_FILE}; a map has no files",
                    piece.code
                )));
            }
        }
        Ok(())
    }

    /// Validates that every move's step covers some distance. A repeating move with no
    /// distance stays on its square forever. A map has no vector steps (see
    /// `validate_vector_sizes`).
    fn validate_steps(&self) -> Result<(), GameSpecError> {
        if self.board.map.is_some() {
            return Ok(());
        }
        for piece in &self.pieces {
            for movement in &piece.moves {
                let StepSpec::Vector(step) = &movement.step else {
                    continue;
                };
                if step.iter().all(|distance| *distance == 0) {
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
    /// has one entry per dimension of the board, none longer than `MAX_STEP`. A map has no
    /// vectors, so on a map any of them is refused.
    fn validate_vector_sizes(&self) -> Result<(), GameSpecError> {
        let expected = self.board.dimensions.len();
        let on_map = self.board.map.is_some();
        let check = |what: String, vector: &[i16]| {
            if on_map {
                return Err(GameSpecError::NotOnAMap(format!(
                    "the {what} is a vector, {vector:?}; a map has places and links, so a \
                     move goes along a label such as `next`"
                )));
            }
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
                if let StepSpec::Vector(step) = &movement.step {
                    check(format!("step of {where_}"), step)?;
                }
                for condition in legacy(&movement.conditions) {
                    check_condition(&where_, condition)?;
                }
                for side_effect in &movement.side_effects {
                    check_side_effect(&where_, side_effect)?;
                }
                for modifier in &movement.modifiers {
                    for condition in legacy(&modifier.conditions) {
                        check_condition(&where_, condition)?;
                    }
                }
                for action in &movement.actions {
                    for condition in legacy(&action.conditions) {
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

    /// Validates every test of a move's, an action's and a modifier's conditions, of a repeat's
    /// `until` and of a piece's drop conditions: each names only places, owners, attackers,
    /// ends, pieces, zones and labels the game has, its bounds are in order, and on a map it
    /// uses no vector (an offset, a `PATH` or a `LINE`). A drop has no `START`.
    fn validate_tests(&self, piece_names: &HashSet<String>) -> Result<(), GameSpecError> {
        for piece in &self.pieces {
            let where_ = format!("the drop conditions of piece {}", piece.code);
            for test in &piece.drop_conditions {
                self.validate_test(test, &where_, false, piece_names)?;
            }
            for movement in &piece.moves {
                let where_ = format!("move {} of piece {}", movement.id, piece.code);
                for test in tests(&movement.conditions) {
                    self.validate_test(test, &where_, true, piece_names)?;
                }
                for action in &movement.actions {
                    let where_ = format!("the {} action of {where_}", action.state);
                    for test in tests(&action.conditions) {
                        self.validate_test(test, &where_, true, piece_names)?;
                    }
                }
                for modifier in &movement.modifiers {
                    let where_ = format!("a modifier of {where_}");
                    for test in tests(&modifier.conditions) {
                        self.validate_test(test, &where_, true, piece_names)?;
                    }
                }
                if let Some(UntilSpec::Test(test)) =
                    movement.repeat.as_ref().and_then(|r| r.until.as_ref())
                {
                    let where_ = format!("the repeat of {where_}");
                    self.validate_test(test, &where_, true, piece_names)?;
                }
            }
        }
        Ok(())
    }

    /// Validates `test`, found in `where_`; `has_start` says whether `START` names a square
    /// there.
    fn validate_test(
        &self,
        test: &TestSpec,
        where_: &str,
        has_start: bool,
        piece_names: &HashSet<String>,
    ) -> Result<(), GameSpecError> {
        match test {
            TestSpec::All { of } | TestSpec::Any { of } => {
                for test in of {
                    self.validate_test(test, where_, has_start, piece_names)?;
                }
                Ok(())
            }
            TestSpec::Not { of } => self.validate_test(of, where_, has_start, piece_names),
            TestSpec::Count {
                r#in,
                r#where,
                min,
                max,
            } => {
                self.validate_region(r#in, where_, has_start)?;
                if let Some(filter) = r#where {
                    validate_filter(filter, where_, piece_names)?;
                }
                let (min, max) = (min.unwrap_or(1), max.unwrap_or(u16::MAX));
                if min > max {
                    return Err(GameSpecError::InvalidTest(format!(
                        "a COUNT in {where_} has its min above its max: min is {min}, max is \
                         {max} (min is 1 when not written)"
                    )));
                }
                Ok(())
            }
            TestSpec::Attacked { r#in, by } => {
                self.validate_region(r#in, where_, has_start)?;
                match by.as_deref() {
                    None | Some(ENEMY) => Ok(()),
                    Some(by) => Err(GameSpecError::InvalidTest(format!(
                        "an ATTACKED in {where_} is by `{by}`; the only attacker is ENEMY"
                    ))),
                }
            }
            TestSpec::InZone { at, zone } => {
                self.validate_place(at, where_, has_start)?;
                if !self.declares_zone(zone) {
                    return Err(GameSpecError::InvalidTest(format!(
                        "an IN_ZONE in {where_} names the zone `{zone}`, which no zone or \
                         POSITION condition declares"
                    )));
                }
                Ok(())
            }
        }
    }

    fn validate_region(
        &self,
        region: &RegionSpec,
        where_: &str,
        has_start: bool,
    ) -> Result<(), GameSpecError> {
        let on_map = self.board.map.is_some();
        match region {
            RegionSpec::At { at } => self.validate_place(at, where_, has_start),
            RegionSpec::Path { from, to, ends } => {
                if on_map {
                    return Err(GameSpecError::NotOnAMap(format!(
                        "a PATH in {where_}; a map has no straight lines"
                    )));
                }
                self.validate_place(from, where_, has_start)?;
                self.validate_place(to, where_, has_start)?;
                match ends.iter().find(|end| *end != FROM && *end != TO) {
                    Some(end) => Err(GameSpecError::InvalidTest(format!(
                        "a PATH in {where_} has the end `{end}`; its ends are {FROM} and {TO}"
                    ))),
                    None => Ok(()),
                }
            }
            RegionSpec::Around { of, through } => {
                self.validate_place(of, where_, has_start)?;
                match through.as_deref() {
                    None | Some(ADJACENT) | Some(ORTHOGONAL) => Ok(()),
                    Some(label) if on_map && MAP_LABELS.contains(&label) => Ok(()),
                    Some(label) if on_map => Err(GameSpecError::UnknownLabel {
                        what: format!("AROUND in {where_}"),
                        label: label.to_string(),
                    }),
                    Some(label) => Err(GameSpecError::InvalidTest(format!(
                        "an AROUND in {where_} goes through `{label}`; a grid's neighbourhoods \
                         are {ADJACENT} and {ORTHOGONAL}"
                    ))),
                }
            }
            RegionSpec::Line { through, along } => {
                if on_map {
                    return Err(GameSpecError::NotOnAMap(format!(
                        "a LINE in {where_}; a map has no straight lines"
                    )));
                }
                self.validate_place(through, where_, has_start)?;
                self.validate_test_vector(format!("along of a LINE in {where_}"), along)?;
                if along.iter().all(|&value| value == 0) {
                    return Err(GameSpecError::InvalidTest(format!(
                        "a LINE in {where_} goes along {along:?}, which goes nowhere"
                    )));
                }
                if along.iter().any(|value| value.unsigned_abs() > 1) {
                    return Err(GameSpecError::InvalidTest(format!(
                        "a LINE in {where_} goes along {along:?}; a straight line steps by -1, \
                         0 or 1 on each axis"
                    )));
                }
                Ok(())
            }
        }
    }

    fn validate_place(
        &self,
        place: &PlaceSpec,
        where_: &str,
        has_start: bool,
    ) -> Result<(), GameSpecError> {
        if place.place != START && place.place != TARGET {
            return Err(GameSpecError::InvalidTest(format!(
                "{where_} names the place `{}`; a place is {START} or {TARGET}",
                place.place
            )));
        }
        if place.place == START && !has_start {
            return Err(GameSpecError::InvalidTest(format!(
                "{where_} names the place {START}; a drop has no moving piece, so its only \
                 place is {TARGET}"
            )));
        }
        match &place.offset {
            Some(offset) => {
                self.validate_test_vector(format!("offset of a place in {where_}"), offset)
            }
            None => Ok(()),
        }
    }

    /// A vector of a test has one entry per dimension of the board, none longer than
    /// `MAX_STEP`; a map has no vectors.
    fn validate_test_vector(&self, what: String, vector: &[i16]) -> Result<(), GameSpecError> {
        if self.board.map.is_some() {
            return Err(GameSpecError::NotOnAMap(format!(
                "the {what} is a vector, {vector:?}; a map has places and links, not vectors"
            )));
        }
        let expected = self.board.dimensions.len();
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

    /// No zone name is declared twice, under `zones` or beside a condition of `conditions`,
    /// which shares the zones' names.
    fn validate_zone_names(&self) -> Result<(), GameSpecError> {
        let mut named: HashSet<&str> = self.conditions.iter().map(|c| c.code.as_str()).collect();
        match self.zones.iter().find(|zone| !named.insert(&zone.name)) {
            Some(zone) => Err(GameSpecError::DuplicateZone(zone.name.clone())),
            None => Ok(()),
        }
    }

    /// Whether `name` is a zone: one under `zones`, or a POSITION condition.
    fn declares_zone(&self, name: &str) -> bool {
        self.zones.iter().any(|zone| zone.name == name)
            || self
                .conditions
                .iter()
                .any(|c| c.code == name && c.r#type == POSITION)
    }

    /// A CONNECTION win condition needs a known neighbourhood, zones that are declared (under
    /// `zones` or as POSITION conditions), and an `at_least` between 1 and the number of zones.
    fn validate_connections(&self) -> Result<(), GameSpecError> {
        let refused = |why: String| Err(GameSpecError::InvalidWinCondition(why));
        for rule in self
            .win_conditions
            .iter()
            .filter(|wc| wc.r#type == CONNECTION)
        {
            match rule.through.as_deref() {
                Some(ORTHOGONAL) | Some(ADJACENT) => {}
                Some(other) => return refused(format!("unknown neighbourhood `{other}`")),
                None => return refused("`through` is required".to_string()),
            }
            if rule.zones.is_empty() {
                return refused("a connection needs at least one zone".to_string());
            }
            let mut named = HashSet::new();
            for zone in &rule.zones {
                if !named.insert(zone) {
                    return refused(format!("zone {zone} is named twice"));
                }
                if !self.declares_zone(zone) {
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

/// The places of `zone` written as names, sorted.
fn written_names(zone: &ZoneSpec) -> Vec<&String> {
    let mut names: Vec<&String> = zone
        .places
        .values()
        .flatten()
        .filter_map(|place| match place {
            PositionSpec::Name(name) => Some(name),
            PositionSpec::Coordinates(_) => None,
        })
        .collect();
    names.sort();
    names
}

/// The conditions of a list written as before tests.
fn legacy(entries: &[ConditionEntry]) -> impl Iterator<Item = &PieceConditionSpec> {
    entries.iter().filter_map(|entry| match entry {
        ConditionEntry::Legacy(condition) => Some(condition),
        ConditionEntry::Test(_) => None,
    })
}

/// The tests of a conditions list.
fn tests(entries: &[ConditionEntry]) -> impl Iterator<Item = &TestSpec> {
    entries.iter().filter_map(|entry| match entry {
        ConditionEntry::Test(test) => Some(test),
        ConditionEntry::Legacy(_) => None,
    })
}

/// A filter's owner is one the tests know, and each of its codes is a piece of the game.
fn validate_filter(
    filter: &FilterSpec,
    where_: &str,
    piece_names: &HashSet<String>,
) -> Result<(), GameSpecError> {
    if let Some(owner) = filter.owner.as_deref() {
        if owner != ALLY && owner != ENEMY && owner != ANY {
            return Err(GameSpecError::InvalidTest(format!(
                "a filter in {where_} has the owner `{owner}`; an owner is {ALLY}, {ENEMY} or \
                 {ANY}"
            )));
        }
    }
    match filter
        .codes
        .iter()
        .find(|code| !piece_names.contains(*code))
    {
        Some(code) => Err(GameSpecError::InvalidTest(format!(
            "a filter in {where_} names the piece `{code}`, which the game does not have"
        ))),
        None => Ok(()),
    }
}
