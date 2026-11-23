use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io;
use thiserror::Error;

use crate::shared::{Direction, ADJACENT, ALLY, CONNECTION, ENEMY, ORTHOGONAL, PIECE_IN_ZONE};
use crate::specs::Validate;

use super::action::{ActionSpec, ChoiceSpec, CodeSpec, DoSpec, EffectSpec, IfSpec};
use super::board::{BoardSpec, PlayerSpec, TurnSpec, MAX_STEP};
use super::draw_conditions::DrawConditionsSpec;
use super::liberty_capture::LibertyCaptureSpec;
use super::map::MAP_LABELS;
use super::old_forms::{self, OldField};
use super::piece::{MoveSpec, PieceSpec, StepSpec};
use super::player::{is_enabled_square, PositionSpec};
use super::test_block::{
    AroundOf, FilterSpec, PlaceSpec, RegionSpec, TestSpec, ANY, EACH, FROM, MOVER, START, TARGET,
    TO,
};
use super::win_condition::WinConditionSpec;
use super::zone::ZoneSpec;

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

    /// The top-level `conditions` list, replaced by `zones`: refused when written.
    #[serde(
        rename = "conditions",
        default,
        skip_serializing,
        deserialize_with = "old_forms::top_level_conditions"
    )]
    pub old_conditions: OldField,

    /// Named zones: for each player, the places of their part. Read by `IN_ZONE` tests and win
    /// conditions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
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

    /// `allow_pass`, replaced by an action with an empty `do`: refused when written.
    #[serde(
        rename = "allow_pass",
        default,
        skip_serializing,
        deserialize_with = "old_forms::allow_pass"
    )]
    pub old_allow_pass: OldField,

    /// When `true`, a move or placement may not leave the position (pieces and player to move)
    /// as it was after any earlier turn. This is the ko rule in Go. Defaults to `false`; for a
    /// draw on repetition instead, see `draw_conditions.repetition_count`.
    #[serde(default)]
    pub forbid_repeated_positions: bool,

    /// Captures groups that have no empty point next to them. Absent means no such captures.
    #[serde(default)]
    pub liberty_capture: Option<LibertyCaptureSpec>,

    /// What a player may do on their turn. A game has at least one action.
    pub actions: Vec<ActionSpec>,
}

fn default_pieces() -> Vec<PieceSpec> {
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

    /// A rule written in a form the spec format no longer has; the message names what replaced
    /// it.
    #[error("Written in a form that is no longer read: {0}")]
    OldForm(String),
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
        self.validate_zone_names(&player_names)?;
        self.validate_win_zones()?;
        self.validate_label_steps()?;
        self.validate_map_positions()?;
        self.validate_steps()?;
        self.validate_vector_sizes()?;
        self.validate_tests(&piece_names)?;
        self.validate_actions(&piece_names, &player_names)?;

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
                    self.validate_test(test, &where_, Scope::of_move(movement), piece_names)?;
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
            TestSpec::Attacked { r#in, by } => {
                self.validate_region(r#in, where_, scope, piece_names)?;
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
            TestSpec::Removed { r#where } => validate_filter(r#where, where_, piece_names),
            TestSpec::Repeats {} => {
                if self.jump_enabled {
                    return Err(GameSpecError::InvalidTest(format!(
                        "a REPEATS in {where_}, in a game with `jump_enabled`; the jump lock \
                         works out the ways again in the middle of a turn, where the turn is not \
                         handed on, so REPEATS cannot be judged"
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
                if scope.steps && scope.movement.is_none() {
                    return Err(GameSpecError::InvalidStep(format!(
                        "a PATH in {where_}; an action has no step to fix the PATH's line when \
                         the spec is read, so a PATH is written only in a piece's move"
                    )));
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
            Some(label) if on_map && MAP_LABELS.contains(&label) => Ok(()),
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
        match place.place.as_str() {
            START if scope.start == Start::Unset => {
                return refused(format!(
                    "{where_} names the place {START}, which no earlier choice sets; a PIECE or \
                     HAND choice sets it"
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
            START | TARGET | EACH => {}
            other => {
                return refused(format!(
                    "{where_} names the place `{other}`; a place is {START}, {TARGET} or {EACH}"
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
        }
        for piece in &self.pieces {
            for movement in &piece.moves {
                let where_ = format!("move {} of piece {}", movement.id, piece.code);
                let mut set = Set::new(Scope::of_move_steps(movement));
                self.validate_steps_of(&movement.r#do, &where_, &mut set, piece_names)?;
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
            ChoiceSpec::Hand { r#where, .. } => {
                if let Some(filter) = r#where {
                    if filter.owner.is_some()
                        || filter.flag.is_some()
                        || filter.moved.is_some()
                        || filter.empty
                    {
                        return Err(GameSpecError::InvalidStep(format!(
                            "{where_} is a HAND choice whose filter has more than `codes`; a \
                             piece in the hand is told apart only by its code"
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
            EffectSpec::Move { from, to } => {
                self.validate_place(from, where_, scope)?;
                self.validate_place(to, where_, scope)
            }
            EffectSpec::Remove { at }
            | EffectSpec::SetOwner { at }
            | EffectSpec::SetFlag { at, .. } => self.validate_place(at, where_, scope),
            EffectSpec::Place { at, code } | EffectSpec::SetCode { at, code } => {
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
            EffectSpec::EndTurn {} => Ok(()),
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

    /// A PIECE_IN_ZONE win condition names a zone declared under `zones`. A CONNECTION win
    /// condition needs a known neighbourhood, zones declared under `zones`, and an `at_least`
    /// between 1 and the number of zones.
    fn validate_win_zones(&self) -> Result<(), GameSpecError> {
        let refused = |why: String| Err(GameSpecError::InvalidWinCondition(why));
        for rule in self
            .win_conditions
            .iter()
            .filter(|wc| wc.r#type == PIECE_IN_ZONE)
        {
            match rule.zone.as_deref() {
                Some(zone) if self.declares_zone(zone) => {}
                Some(zone) => {
                    return refused(format!("{PIECE_IN_ZONE} names the unknown zone {zone}"))
                }
                None => return refused(format!("{PIECE_IN_ZONE}: `zone` is required")),
            }
        }
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
        ChoiceSpec::Hand { .. } => "HAND",
        ChoiceSpec::Code { .. } => "CODE",
    }
}

/// A code written in `where_` is a piece of the game, and a chosen code names an earlier
/// `CODE` choice that `set` holds.
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
        .find(|code| !piece_names.contains(*code))
    {
        Some(code) => Err(GameSpecError::InvalidTest(format!(
            "a filter in {where_} names the piece `{code}`, which the game does not have"
        ))),
        None => Ok(()),
    }
}
