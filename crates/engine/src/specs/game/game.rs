use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io;
use thiserror::Error;

use crate::shared::{Direction, ADJACENT, ALLY, ENEMY, ORTHOGONAL};
use crate::specs::Validate;

use super::action::{
    ActionSpec, ChoiceSpec, CodeSpec, CountSpec, DoSpec, EffectSpec, IfSpec, ALL_PLAYERS,
    MOVER_ONLY,
};
use super::bag::{BagFor, BagRef, BagSpec};
use super::board::{BoardSpec, PlayerSpec, TurnSpec, MAX_STEP};
use super::end_check::{EndCheckSpec, OutcomeSpec, ScoreTermSpec, Viewpoint};
use super::piece::{MoveSpec, PieceSpec, StepSpec};
use super::player::{is_enabled_square, PositionSpec};
use super::rule_steps::EveryActionSpec;
use super::test_block::{
    AroundOf, FilterSpec, PlaceSpec, RegionSpec, TestSpec, ANY, EACH, FROM, LAST, MOVER, START,
    TARGET, TO,
};
use super::zone::ZoneSpec;

/// Full spec of a game, to be read from a .json file.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GameSpec {
    /// The version of the spec format this spec is written for. Required; this engine reads
    /// `FORMAT_VERSION`. The parser checks it before anything else is read.
    pub format_version: u32,

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

    /// Named zones: for each player, the places of their part. Read by `IN_ZONE` tests.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<ZoneSpec>,

    /// The names of the game's counters: each player holds a number for each, starting at 0,
    /// changed by the `ADD` and `RESET` effects and read by `COUNTER` tests and score terms.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub counters: Vec<String>,

    /// Checks judged in order after every turn, once it has passed to the next player: the first
    /// whose test holds ends the game, or removes a player (see `EndCheckSpec`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub end_checks: Vec<EndCheckSpec>,

    /// The game's score: terms summed for each player, read by the `HIGHEST` outcome and shown
    /// live.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub score: Vec<ScoreTermSpec>,

    /// The bags: holders off the board. A game with a hand declares the bag `HAND` for each
    /// player. What goes into one is written on the effects (`removed_to`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bags: Vec<BagSpec>,

    /// What a player may do on their turn. A game has at least one action.
    pub actions: Vec<ActionSpec>,

    /// Steps run after the own steps of every action they apply to (check, a liberty rule).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub every_action: Vec<EveryActionSpec>,

    /// Tests that narrow the ways in the order written: if some way passes one, only the ways
    /// that pass stay; if none does, nothing changes (a forced capture).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub must: Vec<TestSpec>,
}

/// The most faces a die may have.
pub const MAX_FACES: usize = 1000;

/// The version of the spec format this engine reads. A change to what an existing spec means,
/// or the removal of a form that was valid, makes a new version; adding a block, field or form
/// does not.
pub const FORMAT_VERSION: u32 = 1;

fn default_pieces() -> Vec<PieceSpec> {
    vec![]
}

#[derive(Error, Debug)]
pub enum GameSpecError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// The spec does not say which version of the format it is written for.
    #[error("The spec has no format_version; this engine reads format_version {FORMAT_VERSION}")]
    MissingFormatVersion,

    /// The spec says it is written for a version this engine does not read. Carries the value as
    /// written (`2`, `"1"`, `null`).
    #[error("The spec says format_version {0}; this engine reads format_version {FORMAT_VERSION}")]
    UnsupportedFormatVersion(String),

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

    /// A rule names a piece the game does not have.
    #[error("Unknown piece name in a rule: {0}")]
    UnknownPieceInRule(String),

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

    /// A zone name is declared twice under `zones`.
    #[error("Duplicate zone name: {0}")]
    DuplicateZone(String),

    /// A zone has places for a name that is not a player of the game.
    #[error("The zone {zone} has places for {player}, who is not a player of the game")]
    UnknownPlayerInZone { zone: String, player: String },

    /// A step of an action or a move is not well formed or not well placed; the message says
    /// which and where.
    #[error("Invalid step: {0}")]
    InvalidStep(String),
}

impl Validate for GameSpec {
    type Arg1 = ();
    type Arg2 = ();

    /// Validates the consistency of the game spec.
    fn validate(&self, _1: &(), _2: &()) -> Result<(), GameSpecError> {
        if self.format_version != FORMAT_VERSION {
            return Err(GameSpecError::UnsupportedFormatVersion(
                self.format_version.to_string(),
            ));
        }
        self.board.validate_size()?;

        let player_names: HashSet<String> = self.players.iter().map(|p| p.name.clone()).collect();
        let piece_names: HashSet<String> = self.pieces.iter().map(|p| p.code.clone()).collect();

        self.validate_captured_as(&piece_names)?;
        self.validate_bags(&piece_names)?;
        self.validate_holder_places(&piece_names)?;
        self.validate_player_specs(&piece_names)?;
        self.validate_player_name_duplicates(&player_names)?;
        self.validate_players_in_turn_order(&player_names)?;
        self.validate_zone_names(&player_names)?;
        self.validate_label_steps()?;
        self.validate_map_positions()?;
        self.validate_steps()?;
        self.validate_vector_sizes()?;
        self.validate_counters()?;
        self.validate_end_checks(&piece_names, &player_names)?;
        self.validate_stages(&piece_names)?;
        self.validate_tests(&piece_names)?;
        self.validate_actions(&piece_names, &player_names)?;

        Ok(())
    }
}

impl GameSpec {
    pub fn validate_specs(&self) -> Result<(), GameSpecError> {
        self.validate(&(), &())
    }

    /// Validates each piece's `captured_as`: it names a piece of the game.
    fn validate_captured_as(&self, piece_names: &HashSet<String>) -> Result<(), GameSpecError> {
        for piece in &self.pieces {
            if let Some(code) = piece
                .captured_as
                .iter()
                .find(|code| !piece_names.contains(*code))
            {
                return Err(GameSpecError::UnknownPieceInRule(format!(
                    "{code}, in the `captured_as` of {}",
                    piece.code
                )));
            }
        }
        Ok(())
    }

    /// Validates `end_checks` and `score`: each test names only what the game has and no place;
    /// a `STUCK` is judged `as` NEXT; `HIGHEST` has a score to read; each score term is well
    /// formed.
    fn validate_end_checks(
        &self,
        piece_names: &HashSet<String>,
        player_names: &HashSet<String>,
    ) -> Result<(), GameSpecError> {
        for (index, check) in self.end_checks.iter().enumerate() {
            let where_ = format!("entry {} of end_checks", index + 1);
            self.validate_test(&check.when, &where_, Scope::of_end(), piece_names)?;
            if let Some(players) = &check.players {
                if players.is_empty() {
                    return Err(GameSpecError::InvalidTest(format!(
                        "{where_} has an empty `players`; it would apply to no one, so leave \
                         it out to apply to everyone"
                    )));
                }
                if let Some(player) = players.iter().find(|p| !player_names.contains(*p)) {
                    return Err(GameSpecError::InvalidTest(format!(
                        "{where_} is for `{player}`, who is not a player of the game"
                    )));
                }
            }
            if mentions_stuck(&check.when) && check.viewpoint != Viewpoint::Next {
                return Err(GameSpecError::InvalidTest(format!(
                    "{where_} holds a STUCK but is judged `as` MOVER; the player with no way is \
                     the one about to play, so write `\"as\": \"NEXT\"`"
                )));
            }
            if check.then == (OutcomeSpec::Highest {}) && self.score.is_empty() {
                return Err(GameSpecError::InvalidTest(format!(
                    "{where_} ends with HIGHEST, but the game has no `score` to read"
                )));
            }
        }
        for (index, term) in self.score.iter().enumerate() {
            let where_ = format!("term {} of score", index + 1);
            match term {
                ScoreTermSpec::Pieces { r#where } | ScoreTermSpec::Territory { r#where } => {
                    if let Some(filter) = r#where {
                        if filter.owner.is_some() {
                            return Err(GameSpecError::InvalidTest(format!(
                                "{where_} has a filter with `owner`; a score term counts for \
                                 each player in turn, so the pieces are theirs"
                            )));
                        }
                        validate_filter(filter, &where_, piece_names)?;
                    }
                }
                ScoreTermSpec::Kept { zone, r#where } => {
                    if !self.zones.iter().any(|declared| declared.name == *zone) {
                        return Err(GameSpecError::InvalidTest(format!(
                            "{where_} counts the zone `{zone}`, which the game does not have"
                        )));
                    }
                    if let Some(filter) = r#where {
                        if filter.owner.is_some() {
                            return Err(GameSpecError::InvalidTest(format!(
                                "{where_} has a filter with `owner`; a score term counts for \
                                 each player in turn, so the places are theirs"
                            )));
                        }
                        validate_filter(filter, &where_, piece_names)?;
                    }
                }
                ScoreTermSpec::Counter { name } => self.validate_counter_name(name, &where_)?,
                ScoreTermSpec::Points { to, .. } => {
                    if let Some(player) = to.iter().find(|player| !player_names.contains(*player)) {
                        return Err(GameSpecError::InvalidTest(format!(
                            "{where_} gives points to `{player}`, who is not a player of the game"
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Counter names are not empty and none is declared twice.
    fn validate_counters(&self) -> Result<(), GameSpecError> {
        let mut named: HashSet<&str> = HashSet::new();
        for name in &self.counters {
            if name.is_empty() || !named.insert(name) {
                return Err(GameSpecError::InvalidStep(format!(
                    "`counters` names `{name}`, which is empty or declared twice; each counter \
                     has a name of its own"
                )));
            }
        }
        Ok(())
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
                if !self
                    .board
                    .map
                    .as_ref()
                    .is_some_and(|map| map.has_label(label))
                {
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

    /// On a grid, a place of a zone written as a name must name an enabled square. On a map,
    /// every such place is the name of a place.
    fn validate_map_positions(&self) -> Result<(), GameSpecError> {
        let Some(map) = self.board.map.as_deref() else {
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

    /// Validates that every move step has one entry per dimension of the board, none longer
    /// than `MAX_STEP`. A map has no vectors, so on a map any of them is refused.
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
        for piece in &self.pieces {
            for movement in &piece.moves {
                let where_ = format!("move {} of piece {}", movement.id, piece.code);
                if let StepSpec::Vector(step) = &movement.step {
                    check(format!("step of {where_}"), step)?;
                }
            }
        }
        Ok(())
    }

    /// Validates the test of every repeat's `until`: it names only places, owners, attackers,
    /// ends, pieces, zones and labels the game has, its bounds are in order, and on a map it
    /// uses no vector (an offset, a `PATH` or a `LINE`). `EACH` is set only inside a
    /// `FOR_EACH`. A `PATH`'s line is fixed when the spec is read (see `validate_path_line`).
    fn validate_tests(&self, piece_names: &HashSet<String>) -> Result<(), GameSpecError> {
        for piece in &self.pieces {
            for movement in &piece.moves {
                if let Some(test) = movement.repeat.as_ref().and_then(|r| r.until.as_ref()) {
                    let where_ =
                        format!("the repeat of move {} of piece {}", movement.id, piece.code);
                    let scope = Scope {
                        last: false,
                        ..Scope::of_move(movement)
                    };
                    self.validate_test(test, &where_, scope, piece_names)?;
                }
            }
        }
        Ok(())
    }

    /// Validates `test`, found in `where_`, where `scope` says which places are set.
    fn validate_test(
        &self,
        test: &TestSpec,
        where_: &str,
        scope: Scope<'_>,
        piece_names: &HashSet<String>,
    ) -> Result<(), GameSpecError> {
        match test {
            TestSpec::All { of } | TestSpec::Any { of } => {
                for test in of {
                    self.validate_test(test, where_, scope, piece_names)?;
                }
                Ok(())
            }
            TestSpec::Not { of } => self.validate_test(of, where_, scope, piece_names),
            TestSpec::Count {
                r#in,
                r#where,
                min,
                max,
            } => {
                self.validate_region(r#in, where_, scope, piece_names)?;
                if let Some(filter) = r#where {
                    validate_place_filter(filter, where_, piece_names)?;
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
            TestSpec::Attacked {
                r#in,
                by,
                keeping,
                moves,
            } => {
                self.validate_region(r#in, where_, scope, piece_names)?;
                match moves {
                    None | Some(1) => {}
                    Some(2) if keeping.is_some() => {
                        return Err(GameSpecError::InvalidTest(format!(
                            "an ATTACKED in {where_} has `moves` 2 with `keeping`; a look-ahead \
                             of two moves does not simulate captures, so write one or the other"
                        )));
                    }
                    Some(2) => {}
                    Some(other) => {
                        return Err(GameSpecError::InvalidTest(format!(
                            "an ATTACKED in {where_} has `moves` {other}; an attacker looks 1 or 2 \
                             moves ahead"
                        )));
                    }
                }
                if let Some(keeping) = keeping {
                    if keeping.owner.is_some() || keeping.empty {
                        return Err(GameSpecError::InvalidTest(format!(
                            "an ATTACKED in {where_} has a `keeping` filter with `owner` or \
                             `empty`; the pieces kept are the attacker's own, so write codes, \
                             a flag or moved"
                        )));
                    }
                    validate_filter(keeping, where_, piece_names)?;
                }
                match by.as_deref() {
                    None | Some(ENEMY) | Some(MOVER) => Ok(()),
                    Some(by) => Err(GameSpecError::InvalidTest(format!(
                        "an ATTACKED in {where_} is by `{by}`; an attacker is {ENEMY} (the \
                         default) or {MOVER}"
                    ))),
                }
            }
            TestSpec::InZone { at, zone } => {
                self.validate_place(at, where_, scope)?;
                if !self.declares_zone(zone) {
                    return Err(GameSpecError::InvalidTest(format!(
                        "an IN_ZONE in {where_} names the zone `{zone}`, which `zones` does not \
                         declare"
                    )));
                }
                Ok(())
            }
            TestSpec::Removed { .. } | TestSpec::Moved { .. } if scope.ends => {
                Err(GameSpecError::InvalidTest(format!(
                    "a REMOVED or MOVED in {where_}; an end check is judged after a turn, when no \
                     way is being worked out, so there is nothing for them to look at"
                )))
            }
            TestSpec::Same { a, b } => {
                self.validate_place(a, where_, scope)?;
                self.validate_place(b, where_, scope)
            }
            TestSpec::Stuck {} if !scope.ends => Err(GameSpecError::InvalidTest(format!(
                "a STUCK in {where_}; STUCK is judged only by an end check with `as` NEXT"
            ))),
            TestSpec::Stuck {} => Ok(()),
            TestSpec::Removed { r#where } => validate_filter(r#where, where_, piece_names),
            TestSpec::Moved { r#where } => validate_filter(r#where, where_, piece_names),
            TestSpec::SomePlace { r#in, that } => {
                self.validate_region(r#in, where_, scope, piece_names)?;
                let inside = Scope {
                    each: true,
                    ..scope
                };
                self.validate_test(that, where_, inside, piece_names)
            }
            TestSpec::Passes { in_a_row } => {
                if *in_a_row == 0 {
                    return Err(GameSpecError::InvalidTest(format!(
                        "a PASSES in {where_} has `in_a_row` 0, which always holds; write 1 or \
                         more"
                    )));
                }
                Ok(())
            }
            TestSpec::Counter { name, min, max } => {
                self.validate_counter_name(name, where_)?;
                let (min, max) = (min.unwrap_or(0), max.unwrap_or(i64::MAX));
                if min > max {
                    return Err(GameSpecError::InvalidTest(format!(
                        "a COUNTER in {where_} has its min above its max: min is {min}, max is \
                         {max} (min is 0 when not written)"
                    )));
                }
                Ok(())
            }
            TestSpec::Repeats { times } => {
                if *times == Some(0) {
                    return Err(GameSpecError::InvalidTest(format!(
                        "a REPEATS in {where_} has `times` 0, which always holds; write 1 or \
                         more (1 when not written)"
                    )));
                }
                if !self.turns.stages.is_empty() {
                    return Err(GameSpecError::InvalidTest(format!(
                        "a REPEATS in {where_}, in a game with `turns.stages`; a stage works out \
                         the ways again in the middle of a turn, where the turn is not handed \
                         on, so REPEATS cannot be judged"
                    )));
                }
                Ok(())
            }
        }
    }

    /// `name`, found in `where_`, is a counter the game declares under `counters`.
    fn validate_counter_name(&self, name: &str, where_: &str) -> Result<(), GameSpecError> {
        if self.counters.iter().any(|counter| counter == name) {
            return Ok(());
        }
        Err(GameSpecError::InvalidTest(format!(
            "{where_} names the counter `{name}`, which `counters` does not declare"
        )))
    }

    fn validate_region(
        &self,
        region: &RegionSpec,
        where_: &str,
        scope: Scope<'_>,
        piece_names: &HashSet<String>,
    ) -> Result<(), GameSpecError> {
        let on_map = self.board.map.is_some();
        match region {
            RegionSpec::At { at } => self.validate_place(at, where_, scope),
            RegionSpec::Path { from, to, ends } => {
                if on_map {
                    return Err(GameSpecError::NotOnAMap(format!(
                        "a PATH in {where_}; a map has no straight lines"
                    )));
                }
                if scope.movement.is_none() {
                    let refused = format!(
                        "a PATH in {where_}; an action, a stage or an end check has no step to \
                         fix the PATH's line when the spec is read, so a PATH is written only \
                         in a piece's move"
                    );
                    return Err(if scope.steps {
                        GameSpecError::InvalidStep(refused)
                    } else {
                        GameSpecError::InvalidTest(refused)
                    });
                }
                self.validate_place(from, where_, scope)?;
                self.validate_place(to, where_, scope)?;
                if let Some(end) = ends.iter().find(|end| *end != FROM && *end != TO) {
                    return Err(GameSpecError::InvalidTest(format!(
                        "a PATH in {where_} has the end `{end}`; its ends are {FROM} and {TO}"
                    )));
                }
                validate_path_line(from, to, where_, scope.movement)
            }
            RegionSpec::Around { of, through } => {
                match of {
                    AroundOf::Place(place) => self.validate_place(place, where_, scope)?,
                    AroundOf::Region(region) => {
                        self.validate_region(region, where_, scope, piece_names)?
                    }
                }
                self.validate_through("AROUND", through.as_deref(), where_)
            }
            RegionSpec::Group {
                of,
                through,
                r#where,
            } => {
                self.validate_place(of, where_, scope)?;
                self.validate_through("GROUP", through.as_deref(), where_)?;
                match r#where {
                    Some(filter) if filter.empty => Err(GameSpecError::InvalidTest(format!(
                        "a GROUP in {where_} has a filter with `empty`; a GROUP joins pieces, \
                         so its filter is of pieces"
                    ))),
                    Some(filter) => validate_filter(filter, where_, piece_names),
                    None => Ok(()),
                }
            }
            RegionSpec::Follow {
                from,
                steps,
                through,
                skipping,
            } => {
                if !on_map {
                    return Err(GameSpecError::NotOnAMap(format!(
                        "a FOLLOW in {where_} follows a map's links; a grid has none"
                    )));
                }
                self.validate_place(from, where_, scope)?;
                self.validate_count(steps, where_, scope, piece_names, false)?;
                if matches!(steps, CountSpec::Number(number)
                    if *number > crate::shared::MAX_FOLLOW_STEPS)
                {
                    return Err(GameSpecError::InvalidStep(format!(
                        "a FOLLOW in {where_} takes more than {} steps",
                        crate::shared::MAX_FOLLOW_STEPS
                    )));
                }
                let label = through.as_deref().unwrap_or("next");
                self.validate_through("FOLLOW", Some(label), where_)?;
                if let Some(zone) = skipping {
                    if !self.zones.iter().any(|declared| declared.name == *zone) {
                        return Err(GameSpecError::InvalidStep(format!(
                            "a FOLLOW in {where_} skips the zone `{zone}`, which the game does \
                             not have"
                        )));
                    }
                }
                let leads_to_many = self.board.map.as_ref().is_some_and(|map| {
                    map.links
                        .get(label)
                        .is_some_and(|links| links.iter().any(|targets| targets.len() > 1))
                });
                if leads_to_many {
                    return Err(GameSpecError::InvalidStep(format!(
                        "a FOLLOW in {where_} follows `{label}`, which leads to several places \
                         from one; a FOLLOW needs a label with one place to go to"
                    )));
                }
                Ok(())
            }
            RegionSpec::Line { through, along } => {
                if on_map {
                    return Err(GameSpecError::NotOnAMap(format!(
                        "a LINE in {where_}; a map has no straight lines"
                    )));
                }
                self.validate_place(through, where_, scope)?;
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
            RegionSpec::Board { r#where } => match r#where {
                Some(filter) => validate_place_filter(filter, where_, piece_names),
                None => Ok(()),
            },
        }
    }

    /// The neighbourhood `through` of the region `region`, found in `where_`, is one the board
    /// has: `adjacent` (the default) or `orthogonal`, or on a map one of its labels.
    fn validate_through(
        &self,
        region: &str,
        through: Option<&str>,
        where_: &str,
    ) -> Result<(), GameSpecError> {
        let on_map = self.board.map.is_some();
        match through {
            None | Some(ADJACENT) | Some(ORTHOGONAL) => Ok(()),
            Some(label)
                if self
                    .board
                    .map
                    .as_ref()
                    .is_some_and(|map| map.has_label(label)) =>
            {
                Ok(())
            }
            Some(label) if on_map => Err(GameSpecError::UnknownLabel {
                what: format!("{region} in {where_}"),
                label: label.to_string(),
            }),
            Some(label) => {
                let article = if region == "AROUND" { "an" } else { "a" };
                Err(GameSpecError::InvalidTest(format!(
                    "{article} {region} in {where_} goes through `{label}`; a grid's \
                     neighbourhoods are {ADJACENT} and {ORTHOGONAL}"
                )))
            }
        }
    }

    /// Validates `place`, found in `where_`: it is one `scope` sets, and its offset is a
    /// vector of the board. In steps, a refusal is of a step, elsewhere of a test.
    fn validate_place(
        &self,
        place: &PlaceSpec,
        where_: &str,
        scope: Scope<'_>,
    ) -> Result<(), GameSpecError> {
        let refused = |why: String| {
            Err(if scope.steps {
                GameSpecError::InvalidStep(why)
            } else {
                GameSpecError::InvalidTest(why)
            })
        };
        if let Some(bag) = &place.bag {
            if !place.place.is_empty() || place.offset.is_some() {
                return refused(format!(
                    "{where_} names a bag and also a `place` or an `offset`; a bag is written \
                     {{ \"bag\": ..., \"of\": ... }} alone"
                ));
            }
            let bag = BagRef {
                bag: bag.clone(),
                of: place.of.clone(),
            };
            return self.validate_bag_ref(&bag, where_);
        }
        if place.of.is_some() {
            return refused(format!("{where_} has an `of` but names no bag"));
        }
        match place.place.as_str() {
            START if scope.start == Start::Unset => {
                return refused(format!(
                    "{where_} names the place {START}, which no earlier choice sets; a PIECE or \
                     HELD choice sets it"
                ));
            }
            TARGET if !scope.target => {
                return refused(format!(
                    "{where_} names the place {TARGET}, which no earlier choice sets; a MOVE or \
                     PLACE choice sets it"
                ));
            }
            EACH if !scope.each => {
                return refused(format!(
                    "{where_} names the place {EACH}, which is set only inside the `do` of a \
                     FOR_EACH"
                ));
            }
            LAST if !scope.last => {
                return refused(format!(
                    "{where_} names the place {LAST}, which only the steps of an action or of a \
                     move's `do` and a stage's `only` can name"
                ));
            }
            START | TARGET | EACH | LAST => {}
            other => {
                return refused(format!(
                    "{where_} names the place `{other}`; a place is {START}, {TARGET}, {EACH} or \
                     {LAST}"
                ));
            }
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

    /// Validates the actions and every move's `do`: action names are unique, their players are
    /// players of the game, and each step is well placed (see `validate_steps_of`). A game has
    /// at least one action.
    fn validate_actions(
        &self,
        piece_names: &HashSet<String>,
        player_names: &HashSet<String>,
    ) -> Result<(), GameSpecError> {
        if self.actions.is_empty() {
            return Err(GameSpecError::InvalidStep(
                "the game has no `actions`; write at least one, such as the action MOVE that \
                 chooses a PIECE and then one of its moves"
                    .to_string(),
            ));
        }
        let mut named: HashSet<&str> = HashSet::new();
        for action in &self.actions {
            if !named.insert(&action.name) {
                return Err(GameSpecError::InvalidStep(format!(
                    "the action name `{}` is used twice; each action has a name of its own",
                    action.name
                )));
            }
            if let Some(player) = action.players.iter().find(|p| !player_names.contains(*p)) {
                return Err(GameSpecError::InvalidStep(format!(
                    "action {} is for `{player}`, who is not a player of the game",
                    action.name
                )));
            }
            let where_ = format!("action {}", action.name);
            let mut set = Set::new(Scope::of_action());
            self.validate_steps_of(&action.r#do, &where_, &mut set, piece_names)?;
            if let Some(given) = gives_chance_away(&action.r#do, &mut false) {
                return Err(GameSpecError::InvalidStep(format!(
                    "{where_} {given} after a ROLL or a SHUFFLE; the ways of an action are known \
                     before the player picks it, so the options would show what was rolled. Put \
                     the choices and tests first, then the chance, and hand on with STAGE for \
                     anything the player decides after seeing it"
                )));
            }
            // The entries run after the action's own steps, on what they have set.
            for (index, entry) in self.every_action.iter().enumerate() {
                if entry.applies_to(&action.name) {
                    let where_ = format!(
                        "entry {} of every_action, run after action {}",
                        index + 1,
                        action.name
                    );
                    self.validate_steps_of(&entry.r#do, &where_, &mut set, piece_names)?;
                }
            }
        }
        self.validate_every_action(&named)?;
        self.validate_must(piece_names)?;
        for piece in &self.pieces {
            for movement in &piece.moves {
                let where_ = format!("move {} of piece {}", movement.id, piece.code);
                let mut set = Set::new(Scope::of_move_steps(movement));
                self.validate_steps_of(&movement.r#do, &where_, &mut set, piece_names)?;
            }
        }
        Ok(())
    }

    /// Validates what `every_action` says apart from where its steps stand: each `for` names
    /// actions the game has, and no entry asks the player anything.
    fn validate_every_action(&self, actions: &HashSet<&str>) -> Result<(), GameSpecError> {
        for (index, entry) in self.every_action.iter().enumerate() {
            let where_ = format!("entry {} of every_action", index + 1);
            if let Some(names) = &entry.r#for {
                if names.is_empty() {
                    return Err(GameSpecError::InvalidStep(format!(
                        "{where_} has an empty `for`, which applies to no action; leave `for` \
                         out to apply it to every action"
                    )));
                }
                if let Some(name) = names.iter().find(|name| !actions.contains(name.as_str())) {
                    return Err(GameSpecError::InvalidStep(format!(
                        "{where_} is for the action `{name}`, which the game does not have"
                    )));
                }
            }
            if entry.r#do.iter().any(asks_the_player) {
                return Err(GameSpecError::InvalidStep(format!(
                    "{where_} holds a choose step; rules that apply to every move run without \
                     asking the player, so write tests, effects and `if`s"
                )));
            }
        }
        Ok(())
    }

    /// Validates the tests of `must`. They are judged on a way as it ended, so they name no
    /// place: `START`, `TARGET` and `EACH` are refused.
    fn validate_must(&self, piece_names: &HashSet<String>) -> Result<(), GameSpecError> {
        for (index, test) in self.must.iter().enumerate() {
            let where_ = format!("entry {} of must", index + 1);
            let scope = Scope {
                last: false,
                ..Scope::of_action()
            };
            self.validate_test(test, &where_, scope, piece_names)?;
        }
        Ok(())
    }

    /// Validates `turns.stages`: each name is written once, and each `only` is a test over the
    /// way's own piece and the place the way before ended.
    fn validate_stages(&self, piece_names: &HashSet<String>) -> Result<(), GameSpecError> {
        let mut seen = HashSet::new();
        for stage in &self.turns.stages {
            if stage.name.is_empty() || !seen.insert(stage.name.as_str()) {
                return Err(GameSpecError::InvalidStep(format!(
                    "the stage `{}` of `turns.stages` has no name or shares it with another stage",
                    stage.name
                )));
            }
            if let Some(only) = &stage.only {
                let where_ = format!("the `only` of stage {}", stage.name);
                self.validate_test(only, &where_, Scope::of_stage(), piece_names)?;
            }
        }
        Ok(())
    }

    /// Validates `steps`, found in `where_`, in order: each names only what the game has, and
    /// only places and chosen codes that `set` holds, as the steps before it leave it. The two
    /// branches of an `if` each start from the set before it, and what is set after it is what
    /// both set.
    fn validate_steps_of(
        &self,
        steps: &[DoSpec],
        where_: &str,
        set: &mut Set<'_>,
        piece_names: &HashSet<String>,
    ) -> Result<(), GameSpecError> {
        for (index, step) in steps.iter().enumerate() {
            let where_ = format!("step {} of {where_}", index + 1);
            match step {
                DoSpec::Choose(choice) => {
                    self.validate_choice(choice, &where_, set, piece_names)?
                }
                DoSpec::Test(_) if set.scope.each => {
                    return Err(GameSpecError::InvalidStep(format!(
                        "{where_} is a test inside a FOR_EACH; a FOR_EACH holds effects and \
                         `if`s, so write the test as the `if` of an `if`"
                    )));
                }
                DoSpec::Test(test) => self.validate_test(test, &where_, set.scope, piece_names)?,
                DoSpec::Effect(effect) => {
                    self.validate_effect(effect, &where_, set, piece_names)?
                }
                DoSpec::If(IfSpec { r#if, then, r#else }) => {
                    self.validate_test(r#if, &where_, set.scope, piece_names)?;
                    let mut after_then = set.clone();
                    let then_where = format!("the `then` of {where_}");
                    self.validate_steps_of(then, &then_where, &mut after_then, piece_names)?;
                    let mut after_else = set.clone();
                    let else_where = format!("the `else` of {where_}");
                    self.validate_steps_of(r#else, &else_where, &mut after_else, piece_names)?;
                    *set = after_then.meet(&after_else);
                }
            }
        }
        Ok(())
    }

    /// Validates `choice`, found in `where_`, and notes in `set` what it sets.
    fn validate_choice(
        &self,
        choice: &ChoiceSpec,
        where_: &str,
        set: &mut Set<'_>,
        piece_names: &HashSet<String>,
    ) -> Result<(), GameSpecError> {
        if set.scope.each {
            return Err(GameSpecError::InvalidStep(format!(
                "{where_} is a {} choice inside a FOR_EACH; a FOR_EACH runs without asking the \
                 player, so it holds no choices",
                choice_kind(choice)
            )));
        }
        if set.scope.movement.is_some() && !matches!(choice, ChoiceSpec::Code { .. }) {
            return Err(GameSpecError::InvalidStep(format!(
                "{where_} is a {} choice inside a piece's move; a move's steps run with {START} \
                 the piece's square and {TARGET} one of its targets, so the only choice there \
                 is CODE",
                choice_kind(choice)
            )));
        }
        match choice {
            ChoiceSpec::Piece { r#where, .. } => {
                validate_filter(r#where, where_, piece_names)?;
                set.scope.start = Start::Board;
            }
            ChoiceSpec::Move { .. } => {
                if set.scope.start != Start::Board {
                    return Err(GameSpecError::InvalidStep(format!(
                        "{where_} is a MOVE choice, but no earlier PIECE choice picks a piece on \
                         the board to move"
                    )));
                }
                set.scope.target = true;
            }
            ChoiceSpec::Place { r#in, .. } => {
                self.validate_region(r#in, where_, set.scope, piece_names)?;
                set.scope.target = true;
            }
            ChoiceSpec::Held { r#in, r#where, .. } => {
                self.validate_bag_ref(r#in, where_)?;
                if self
                    .bags
                    .iter()
                    .any(|bag| bag.name == r#in.bag && bag.ordered)
                {
                    return Err(GameSpecError::InvalidStep(format!(
                        "{where_} is a HELD choice in the ordered bag `{}`; a pile has no \
                         \"pick one from it\", so draw with a MOVE",
                        r#in.bag
                    )));
                }
                if let Some(filter) = r#where {
                    if filter.owner.is_some()
                        || filter.flag.is_some()
                        || filter.moved.is_some()
                        || !filter.except.is_empty()
                        || filter.empty
                    {
                        return Err(GameSpecError::InvalidStep(format!(
                            "{where_} is a HELD choice whose filter has more than `codes`; a \
                             piece in a bag is told apart only by its code"
                        )));
                    }
                    validate_filter(filter, where_, piece_names)?;
                }
                set.scope.start = Start::Hand;
            }
            ChoiceSpec::Code { r#as, options } => {
                if let Some(code) = options.iter().find(|code| !piece_names.contains(*code)) {
                    return Err(GameSpecError::UnknownPieceInRule(format!(
                        "{code}, in {where_}"
                    )));
                }
                set.codes.insert(r#as.clone());
            }
        }
        Ok(())
    }

    /// Validates the bags: names are unique and plain, capacities and starts make sense, and no
    /// bag has the name of a place of a map.
    fn validate_bags(&self, piece_names: &HashSet<String>) -> Result<(), GameSpecError> {
        let mut names: HashSet<&str> = HashSet::new();
        for bag in &self.bags {
            let invalid = |why: String| {
                Err(GameSpecError::InvalidStep(format!(
                    "bag `{}`: {why}",
                    bag.name
                )))
            };
            if bag.name.is_empty() || bag.name.contains('@') {
                return invalid("a bag's name is not empty and holds no `@`".to_string());
            }
            if !names.insert(bag.name.as_str()) {
                return invalid("two bags have this name".to_string());
            }
            if self
                .board
                .map
                .as_ref()
                .is_some_and(|map| map.places.iter().any(|place| place.name == bag.name))
            {
                return invalid("a place of the map has this name".to_string());
            }
            if bag.capacity == Some(0) {
                return invalid("its capacity is 0".to_string());
            }
            if bag.ordered && bag.name == "HAND" {
                return invalid(
                    "the hand is picked from, so it cannot be ordered; draw into it from a pile"
                        .to_string(),
                );
            }
            validate_start(
                &bag.start,
                bag.capacity,
                piece_names,
                &format!("bag `{}`", bag.name),
            )?;
        }
        Ok(())
    }

    /// Validates the capacity and start of the places of a map: a capacity is above 0, and a
    /// start belongs to a place that holds many (capacity above 1) and fits it.
    fn validate_holder_places(&self, piece_names: &HashSet<String>) -> Result<(), GameSpecError> {
        let Some(map) = &self.board.map else {
            return Ok(());
        };
        for place in &map.places {
            let what = format!("place `{}`", place.name);
            let invalid = |why: &str| Err(GameSpecError::InvalidStep(format!("{what}: {why}")));
            if place.capacity == Some(0) {
                return invalid("its capacity is 0");
            }
            if !place.start.is_empty() && place.capacity.unwrap_or(1) < 2 {
                return invalid("it has a `start` but holds one piece; write a `capacity` above 1");
            }
            validate_start(&place.start, place.capacity, piece_names, &what)?;
        }
        Ok(())
    }

    /// Validates a `removed_to`, or the bag a `HELD` choice looks in, found in `where_`: the game
    /// has that bag, `of` is written exactly for a bag each player has, and names a player.
    fn validate_bag_ref(&self, bag: &BagRef, where_: &str) -> Result<(), GameSpecError> {
        let refused = |why: String| Err(GameSpecError::InvalidStep(format!("{where_}: {why}")));
        let Some(declared) = self.bags.iter().find(|declared| declared.name == bag.bag) else {
            return refused(format!(
                "the bag `{}` is not declared; write it under `bags`, as \
                 {{ \"name\": \"{}\", \"for\": \"EACH_PLAYER\" }}",
                bag.bag, bag.bag
            ));
        };
        match (&bag.of, declared.owners) {
            (None, Some(BagFor::EachPlayer)) => refused(format!(
                "the bag `{}` is each player's, so say whose with `of`: MOVER, ENEMY or a \
                 player's name",
                bag.bag
            )),
            (Some(_), None) => refused(format!(
                "the bag `{}` is shared, so it takes no `of`",
                bag.bag
            )),
            (Some(of), Some(_)) => {
                let known = of == "MOVER"
                    || (of == "ENEMY" && self.players.len() == 2)
                    || self.players.iter().any(|player| player.name == *of);
                if known {
                    Ok(())
                } else {
                    refused(format!(
                        "`of` is `{of}`; it is MOVER, ENEMY (in a game of two players) or a \
                         player's name"
                    ))
                }
            }
            (None, None) => Ok(()),
        }
    }

    /// Whether the game has a holder: a bag, or a map place that holds many.
    fn has_holders(&self) -> bool {
        !self.bags.is_empty()
            || self.board.map.as_ref().is_some_and(|map| {
                map.places
                    .iter()
                    .any(|place| place.capacity.unwrap_or(1) > 1)
            })
    }

    /// Validates the `what` and `count` of an effect found in `where_`: only a game with a holder
    /// has use for them, and `what` is a filter of `codes` and `owner` alone.
    fn validate_what_and_count(
        &self,
        what: Option<&FilterSpec>,
        count: Option<&CountSpec>,
        where_: &str,
        scope: Scope<'_>,
        piece_names: &HashSet<String>,
    ) -> Result<(), GameSpecError> {
        if (what.is_some() || count.is_some()) && !self.has_holders() {
            return Err(GameSpecError::InvalidStep(format!(
                "{where_} has a `what` or a `count`, which pick pieces out of a holder, but the \
                 game has no bag and no place that holds many"
            )));
        }
        if let Some(filter) = what {
            if filter.flag.is_some() || filter.moved.is_some() || filter.empty {
                return Err(GameSpecError::InvalidStep(format!(
                    "{where_} has a `what` with more than `codes`, `except` and `owner`; a \
                     piece in a holder is told apart by its code and owner only"
                )));
            }
            validate_filter(filter, where_, piece_names)?;
        }
        match count {
            Some(count) => self.validate_count(count, where_, scope, piece_names, true),
            None => Ok(()),
        }
    }

    /// Validates a count found in `where_`: what it counts is a region and a filter of the game.
    /// `ALL` is only allowed where `all` is.
    fn validate_count(
        &self,
        count: &CountSpec,
        where_: &str,
        scope: Scope<'_>,
        piece_names: &HashSet<String>,
        all: bool,
    ) -> Result<(), GameSpecError> {
        match count {
            CountSpec::Number(_) => Ok(()),
            CountSpec::All if all => Ok(()),
            CountSpec::All => Err(GameSpecError::InvalidStep(format!(
                "{where_} has a count of ALL, which a number of steps cannot be"
            ))),
            CountSpec::Of(of) => {
                self.validate_region(&of.r#in, where_, scope, piece_names)?;
                match &of.r#where {
                    Some(filter) if !filter.empty => validate_filter(filter, where_, piece_names),
                    Some(_) => Err(GameSpecError::InvalidStep(format!(
                        "{where_} counts the places that are empty; a count is of pieces"
                    ))),
                    None => Ok(()),
                }
            }
        }
    }

    /// Validates `effect`, found in `where_`, with what `set` holds.
    fn validate_effect(
        &self,
        effect: &EffectSpec,
        where_: &str,
        set: &Set<'_>,
        piece_names: &HashSet<String>,
    ) -> Result<(), GameSpecError> {
        let scope = set.scope;
        match effect {
            EffectSpec::Move {
                from,
                to,
                removed_to,
                what,
                count,
            } => {
                if let Some(removed_to) = removed_to {
                    self.validate_bag_ref(removed_to, where_)?;
                }
                self.validate_what_and_count(
                    what.as_ref(),
                    count.as_ref(),
                    where_,
                    scope,
                    piece_names,
                )?;
                self.validate_not_picking_from_a_pile(what.as_ref(), from, where_)?;
                self.validate_place(from, where_, scope)?;
                self.validate_place(to, where_, scope)
            }
            EffectSpec::Remove {
                at,
                removed_to,
                what,
                count,
            } => {
                if let Some(removed_to) = removed_to {
                    self.validate_bag_ref(removed_to, where_)?;
                }
                self.validate_what_and_count(
                    what.as_ref(),
                    count.as_ref(),
                    where_,
                    scope,
                    piece_names,
                )?;
                self.validate_not_picking_from_a_pile(what.as_ref(), at, where_)?;
                self.validate_place(at, where_, scope)
            }
            EffectSpec::SetOwner { at } | EffectSpec::SetFlag { at, .. } => {
                self.validate_place(at, where_, scope)
            }
            EffectSpec::Place { at, code, count } => {
                self.validate_what_and_count(None, count.as_ref(), where_, scope, piece_names)?;
                self.validate_place(at, where_, scope)?;
                validate_code(code, where_, set, piece_names)
            }
            EffectSpec::SetCode { at, code } => {
                self.validate_place(at, where_, scope)?;
                validate_code(code, where_, set, piece_names)
            }
            EffectSpec::ForEach { r#in, r#do } => {
                self.validate_region(r#in, where_, scope, piece_names)?;
                let mut inside = set.clone();
                inside.scope.each = true;
                let where_ = format!("the FOR_EACH of {where_}");
                self.validate_steps_of(r#do, &where_, &mut inside, piece_names)
            }
            EffectSpec::Add { counter, by, to } => {
                self.validate_counter_effect(counter, to.as_deref(), where_)?;
                if *by == Some(0) {
                    return Err(GameSpecError::InvalidStep(format!(
                        "{where_} adds 0 to the counter `{counter}`, which changes nothing; \
                         write a whole number other than 0 (1 when `by` is not written)"
                    )));
                }
                Ok(())
            }
            EffectSpec::Reset { counter, to } => {
                self.validate_counter_effect(counter, to.as_deref(), where_)
            }
            EffectSpec::Roll { at, dice, faces } => {
                self.validate_chance_scope(set, "ROLL", where_)?;
                self.validate_bag_ref(at, where_)?;
                if let Some(dice) = dice {
                    self.validate_count(dice, where_, scope, piece_names, false)?;
                }
                if faces.is_empty() || faces.len() > MAX_FACES {
                    return Err(GameSpecError::InvalidStep(format!(
                        "{where_} rolls dice with {} faces; a die has from 1 to {MAX_FACES}",
                        faces.len()
                    )));
                }
                for face in faces {
                    if face.is_empty() || face.values().any(|count| *count == 0) {
                        return Err(GameSpecError::InvalidStep(format!(
                            "{where_} has a face with no pieces or with a count of 0; a face \
                             is written as {{ \"CODE\": n }} with n above 0"
                        )));
                    }
                    if let Some(code) = face.keys().find(|code| !piece_names.contains(*code)) {
                        return Err(GameSpecError::UnknownPieceInRule(format!(
                            "{code}, in a face of {where_}"
                        )));
                    }
                }
                Ok(())
            }
            EffectSpec::Shuffle { at } => {
                self.validate_chance_scope(set, "SHUFFLE", where_)?;
                self.validate_bag_ref(at, where_)?;
                match self.bags.iter().find(|bag| bag.name == at.bag) {
                    Some(bag) if bag.ordered => Ok(()),
                    _ => Err(GameSpecError::InvalidStep(format!(
                        "{where_} shuffles the bag `{}`, which is not ordered; only a pile has \
                         an order to shuffle, so write `\"ordered\": true` on the bag",
                        at.bag
                    ))),
                }
            }
            EffectSpec::Stage { name } => {
                if self.turns.stages.iter().any(|stage| stage.name == *name) {
                    return Ok(());
                }
                Err(GameSpecError::InvalidStep(format!(
                    "{where_} runs STAGE `{name}`, which `turns.stages` does not declare"
                )))
            }
        }
    }

    /// `ROLL` and `SHUFFLE` belong to actions: a piece's move is also run to see what it attacks,
    /// and chance has no place there.
    fn validate_chance_scope(
        &self,
        set: &Set<'_>,
        effect: &str,
        where_: &str,
    ) -> Result<(), GameSpecError> {
        if set.scope.movement.is_some() {
            return Err(GameSpecError::InvalidStep(format!(
                "{where_} is a {effect} inside a piece's move; chance belongs to the steps of an \
                 action"
            )));
        }
        Ok(())
    }

    /// A pile gives its top pieces, so a `what` that picks among them has no meaning.
    fn validate_not_picking_from_a_pile(
        &self,
        what: Option<&FilterSpec>,
        from: &PlaceSpec,
        where_: &str,
    ) -> Result<(), GameSpecError> {
        if what.is_some() && self.is_pile(from) {
            return Err(GameSpecError::InvalidStep(format!(
                "{where_} takes from the ordered bag `{}` with a `what`; a pile gives its top \
                 pieces, so leave `what` out",
                from.bag.as_deref().unwrap_or_default()
            )));
        }
        Ok(())
    }

    /// Whether `place` names an ordered bag.
    fn is_pile(&self, place: &PlaceSpec) -> bool {
        place.bag.as_ref().is_some_and(|name| {
            self.bags
                .iter()
                .any(|bag| bag.name == *name && bag.ordered)
        })
    }

    /// A counter effect, found in `where_`, names a declared counter and a `to` it knows.
    fn validate_counter_effect(
        &self,
        counter: &str,
        to: Option<&str>,
        where_: &str,
    ) -> Result<(), GameSpecError> {
        if !self.counters.iter().any(|name| name == counter) {
            return Err(GameSpecError::InvalidStep(format!(
                "{where_} changes the counter `{counter}`, which `counters` does not declare"
            )));
        }
        match to {
            None | Some(ALL_PLAYERS) | Some(MOVER_ONLY) => Ok(()),
            Some(other) => Err(GameSpecError::InvalidStep(format!(
                "{where_} is `to` `{other}`; a counter effect is `to` {MOVER_ONLY} (the default) \
                 or {ALL_PLAYERS}"
            ))),
        }
    }

    /// No zone name is declared twice, and every zone has places only for players of the game.
    fn validate_zone_names(&self, player_names: &HashSet<String>) -> Result<(), GameSpecError> {
        let mut named: HashSet<&str> = HashSet::new();
        if let Some(zone) = self.zones.iter().find(|zone| !named.insert(&zone.name)) {
            return Err(GameSpecError::DuplicateZone(zone.name.clone()));
        }
        for zone in &self.zones {
            let mut keys: Vec<&String> = zone.places.keys().collect();
            keys.sort();
            if let Some(player) = keys.into_iter().find(|key| !player_names.contains(*key)) {
                return Err(GameSpecError::UnknownPlayerInZone {
                    zone: zone.name.clone(),
                    player: player.clone(),
                });
            }
        }
        Ok(())
    }

    /// Whether `name` is a zone under `zones`.
    fn declares_zone(&self, name: &str) -> bool {
        self.zones.iter().any(|zone| zone.name == name)
    }

    /// Validates players specs to be valid.
    fn validate_player_specs(&self, piece_names: &HashSet<String>) -> Result<(), GameSpecError> {
        for player in &self.players {
            player.validate(&piece_names, &self.board)?;
        }
        // A place that holds many keeps its pieces in the holder store (`start`); a standing
        // piece there would be counted twice and could not move.
        let holders: HashSet<&str> = self
            .board
            .map
            .iter()
            .flat_map(|map| map.places.iter())
            .filter(|place| place.capacity.unwrap_or(1) > 1)
            .map(|place| place.name.as_str())
            .collect();
        for player in &self.players {
            for starting in &player.starting_positions {
                for position in &starting.positions {
                    let name = position.position();
                    if holders.contains(name.as_str()) {
                        return Err(GameSpecError::InvalidStep(format!(
                            "player {} starts a {} on `{name}`, a place that holds many; use its `start`",
                            player.name, starting.piece
                        )));
                    }
                }
            }
        }

        Ok(())
    }
}

/// Whether `test` holds a `STUCK` anywhere inside it.
fn mentions_stuck(test: &TestSpec) -> bool {
    match test {
        TestSpec::Stuck {} => true,
        TestSpec::All { of } | TestSpec::Any { of } => of.iter().any(mentions_stuck),
        TestSpec::Not { of } => mentions_stuck(of),
        TestSpec::SomePlace { that, .. } => mentions_stuck(that),
        _ => false,
    }
}

/// What `START` names where a block is checked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Start {
    /// Nothing: no choice has set it yet.
    Unset,
    /// A piece on the board.
    Board,
    /// A piece in the mover's hand.
    Hand,
}

/// Whether `step` asks the player something, itself or in an `if` or a `FOR_EACH` it holds.
fn asks_the_player(step: &DoSpec) -> bool {
    match step {
        DoSpec::Choose(_) => true,
        DoSpec::Test(_) => false,
        DoSpec::Effect(EffectSpec::ForEach { r#do, .. }) => r#do.iter().any(asks_the_player),
        DoSpec::Effect(_) => false,
        DoSpec::If(IfSpec { then, r#else, .. }) => then.iter().chain(r#else).any(asks_the_player),
    }
}

/// Which places a block may name where it is checked.
#[derive(Clone, Copy, Debug)]
struct Scope<'a> {
    /// The move the block belongs to, whose step places `TARGET` one step from `START`.
    movement: Option<&'a MoveSpec>,
    start: Start,
    target: bool,
    /// Inside the `do` of a `FOR_EACH`.
    each: bool,
    /// In steps (an action's or a move's `do`), where a place no earlier choice sets is refused
    /// as a step, not as a test.
    steps: bool,
    /// In an end check, judged after a turn, where no way is being worked out.
    ends: bool,
    /// Where `LAST` may be named.
    last: bool,
}

impl<'a> Scope<'a> {
    /// A move's: `START` is the piece's square and `TARGET` where it lands.
    fn of_move(movement: &'a MoveSpec) -> Self {
        Scope {
            movement: Some(movement),
            start: Start::Board,
            target: true,
            each: false,
            steps: false,
            ends: false,
            last: true,
        }
    }

    /// A move's `do`, run with `START` the piece's square and `TARGET` a candidate.
    fn of_move_steps(movement: &'a MoveSpec) -> Self {
        Scope {
            steps: true,
            ..Scope::of_move(movement)
        }
    }

    /// An action's steps, which start with nothing set.
    fn of_action() -> Self {
        Scope {
            movement: None,
            start: Start::Unset,
            target: false,
            each: false,
            steps: true,
            ends: false,
            last: true,
        }
    }

    /// A stage's `only`: `START` is the way's piece, and `LAST` where the way before ended.
    fn of_stage() -> Self {
        Scope {
            movement: None,
            start: Start::Board,
            target: false,
            each: false,
            steps: false,
            ends: false,
            last: true,
        }
    }

    /// An end check's: no place is set, and no way is being worked out.
    fn of_end() -> Self {
        Scope {
            ends: true,
            last: false,
            ..Scope::of_action()
        }
    }
}

/// What the steps before a step have set: places, and the names of `CODE` choices.
#[derive(Clone, Debug)]
struct Set<'a> {
    scope: Scope<'a>,
    codes: HashSet<String>,
}

impl<'a> Set<'a> {
    fn new(scope: Scope<'a>) -> Self {
        Set {
            scope,
            codes: HashSet::new(),
        }
    }

    /// What both `self` and `other` set: what is set after an `if` whose branches left them.
    fn meet(&self, other: &Set<'a>) -> Set<'a> {
        let start = if self.scope.start == other.scope.start {
            self.scope.start
        } else {
            Start::Unset
        };
        Set {
            scope: Scope {
                start,
                target: self.scope.target && other.scope.target,
                ..self.scope
            },
            codes: self.codes.intersection(&other.codes).cloned().collect(),
        }
    }
}

/// The name a spec writes for `choice`.
fn choice_kind(choice: &ChoiceSpec) -> &'static str {
    match choice {
        ChoiceSpec::Piece { .. } => "PIECE",
        ChoiceSpec::Move { .. } => "MOVE",
        ChoiceSpec::Place { .. } => "PLACE",
        ChoiceSpec::Held { .. } => "HELD",
        ChoiceSpec::Code { .. } => "CODE",
    }
}

/// A code written in `where_` is a piece of the game, and a chosen code names an earlier
/// `CODE` choice that `set` holds.
/// What a step after a `ROLL` or a `SHUFFLE` would give away before the player commits to the
/// action: a question (the options it offers depend on the result) or a test (a way that fails
/// is not offered, so its absence tells). `after_chance` says whether chance has come already, and
/// is left saying whether it has by the end of `steps` (an `if` leaves it set when either branch
/// does).
fn gives_chance_away(steps: &[DoSpec], after_chance: &mut bool) -> Option<&'static str> {
    for step in steps {
        match step {
            DoSpec::Choose(_) if *after_chance => return Some("asks the player a question"),
            DoSpec::Test(_) if *after_chance => return Some("has a test"),
            DoSpec::Choose(_) | DoSpec::Test(_) => {}
            DoSpec::Effect(EffectSpec::Roll { .. } | EffectSpec::Shuffle { .. }) => {
                *after_chance = true;
            }
            DoSpec::Effect(EffectSpec::ForEach { r#do, .. }) => {
                if let Some(given) = gives_chance_away(r#do, after_chance) {
                    return Some(given);
                }
            }
            DoSpec::Effect(_) => {}
            DoSpec::If(IfSpec { then, r#else, .. }) => {
                let mut in_then = *after_chance;
                let mut in_else = *after_chance;
                if let Some(given) = gives_chance_away(then, &mut in_then)
                    .or_else(|| gives_chance_away(r#else, &mut in_else))
                {
                    return Some(given);
                }
                *after_chance = in_then || in_else;
            }
        }
    }
    None
}

fn validate_code(
    code: &CodeSpec,
    where_: &str,
    set: &Set<'_>,
    piece_names: &HashSet<String>,
) -> Result<(), GameSpecError> {
    match code {
        CodeSpec::Code(code) if !piece_names.contains(code) => Err(
            GameSpecError::UnknownPieceInRule(format!("{code}, in {where_}")),
        ),
        CodeSpec::Chosen { chosen } if !set.codes.contains(chosen) => {
            Err(GameSpecError::InvalidStep(format!(
                "{where_} takes the code {{ \"chosen\": \"{chosen}\" }}, but no earlier CODE \
                 choice is named `{chosen}`"
            )))
        }
        CodeSpec::Code(_) | CodeSpec::Chosen { .. } => Ok(()),
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

/// A `PATH`'s line is fixed when the spec is read: its direction is that of `to` − `from`, with
/// `TARGET` one step from `START`. So the two ends are not the same place, that direction is a
/// straight line (-1, 0 or +1 on each axis, times a distance), and on a move that repeats a
/// `PATH` between `START` and `TARGET` has no offset: the direction is worked out from the
/// first square, and an offset would turn it wrong on the squares past it. `EACH` lies nowhere
/// fixed, so a `PATH` with it at one end only is refused. `movement` is the move the `PATH`
/// belongs to, `None` in an action's steps (where `START` and `TARGET` are
/// chosen apart, so a `PATH` between them is not checked here).
fn validate_path_line(
    from: &PlaceSpec,
    to: &PlaceSpec,
    where_: &str,
    movement: Option<&MoveSpec>,
) -> Result<(), GameSpecError> {
    let refused = |why: String| {
        Err(GameSpecError::InvalidTest(format!(
            "a PATH in {where_} goes from {} to {}; {why}",
            written(from),
            written(to)
        )))
    };
    if (from.place == EACH) != (to.place == EACH) {
        return refused(format!(
            "{EACH} lies nowhere fixed from the other end, so the line is not fixed when the spec \
             is read"
        ));
    }
    let no_offset = Vec::new();
    let from_offset = from.offset.as_ref().unwrap_or(&no_offset);
    let to_offset = to.offset.as_ref().unwrap_or(&no_offset);
    let between_anchors: Vec<i16> = if from.place == to.place {
        Vec::new()
    } else {
        let Some(movement) = movement else {
            return Ok(());
        };
        let has_offset = |offset: &Vec<i16>| offset.iter().any(|&value| value != 0);
        if movement.repeat.is_some() && (has_offset(from_offset) || has_offset(to_offset)) {
            return refused(format!(
                "on a move that repeats, a PATH between {START} and {TARGET} has no offset, as \
                 its line is fixed when the spec is read, from the first square"
            ));
        }
        let StepSpec::Vector(step) = &movement.step else {
            return Ok(());
        };
        if from.place == START {
            step.clone()
        } else {
            step.iter().map(|value| value.saturating_neg()).collect()
        }
    };
    let axes = between_anchors
        .len()
        .max(from_offset.len())
        .max(to_offset.len());
    let at = |vector: &[i16], axis: usize| vector.get(axis).copied().unwrap_or(0);
    let direction: Vec<i16> = (0..axes)
        .map(|axis| {
            at(&between_anchors, axis)
                .saturating_add(at(to_offset, axis))
                .saturating_sub(at(from_offset, axis))
        })
        .collect();
    let distance = direction.iter().map(|value| value.unsigned_abs()).max();
    match distance {
        None | Some(0) => refused(format!(
            "its two ends are the same place, so it has no line (with {TARGET} one step from \
             {START})"
        )),
        Some(distance)
            if direction
                .iter()
                .any(|value| ![0, distance].contains(&value.unsigned_abs())) =>
        {
            refused(format!(
                "its line goes {direction:?} (with {TARGET} one step from {START}), which is no \
                 straight line; a straight line steps by -1, 0 or +1 on each axis"
            ))
        }
        Some(_) => Ok(()),
    }
}

/// A place as a spec writes it: `START`, or `START + [0, 1]` with an offset.
fn written(place: &PlaceSpec) -> String {
    match &place.offset {
        Some(offset) => format!("{} + {offset:?}", place.place),
        None => place.place.clone(),
    }
}

/// A filter of places (a `COUNT`'s or a `BOARD`'s): `empty` stands alone, and a filter without
/// it is one of pieces (see `validate_filter`).
fn validate_place_filter(
    filter: &FilterSpec,
    where_: &str,
    piece_names: &HashSet<String>,
) -> Result<(), GameSpecError> {
    if !filter.empty {
        return validate_filter(filter, where_, piece_names);
    }
    let others = [
        ("owner", filter.owner.is_some()),
        ("codes", !filter.codes.is_empty()),
        ("except", !filter.except.is_empty()),
        ("flag", filter.flag.is_some()),
        ("moved", filter.moved.is_some()),
    ];
    match others.iter().find(|(_, written)| *written) {
        Some((field, _)) => Err(GameSpecError::InvalidTest(format!(
            "a filter in {where_} has `empty` and `{field}`; `empty` takes the places that hold \
             no piece, so it is written alone"
        ))),
        None => Ok(()),
    }
}

/// A filter of pieces: its owner is one the tests know, each of its codes is a piece of the
/// game, and it has no `empty`, which is of places.
fn validate_filter(
    filter: &FilterSpec,
    where_: &str,
    piece_names: &HashSet<String>,
) -> Result<(), GameSpecError> {
    if filter.empty {
        return Err(GameSpecError::InvalidTest(format!(
            "a filter in {where_} has `empty`; it takes places, so it is written only in a \
             COUNT's or a BOARD's `where`"
        )));
    }
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
        .chain(&filter.except)
        .find(|code| !piece_names.contains(*code))
    {
        Some(code) => Err(GameSpecError::InvalidTest(format!(
            "a filter in {where_} names the piece `{code}`, which the game does not have"
        ))),
        None => Ok(()),
    }
}

/// Validates a starting content, `start`, of `what`: known pieces, counts above 0, within the
/// capacity (a capacity of 1 holds no count: a place of one holds a piece, which is not a count).
pub(crate) fn validate_start(
    start: &std::collections::BTreeMap<String, u32>,
    capacity: Option<u32>,
    piece_names: &HashSet<String>,
    what: &str,
) -> Result<(), GameSpecError> {
    let invalid = |why: String| Err(GameSpecError::InvalidStep(format!("{what}: {why}")));
    for (code, count) in start {
        if !piece_names.contains(code) {
            return Err(GameSpecError::UnknownPieceInRule(format!(
                "{code}, in the `start` of {what}"
            )));
        }
        if *count == 0 {
            return invalid(format!("the `start` holds 0 of {code}"));
        }
    }
    let total: u64 = start.values().map(|count| u64::from(*count)).sum();
    if capacity.is_some_and(|capacity| total > u64::from(capacity)) {
        return invalid(format!("the `start` holds {total}, more than its capacity"));
    }
    Ok(())
}
