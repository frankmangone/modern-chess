mod build;
mod generate;

pub(crate) use build::direction_of;

use std::collections::HashMap;

use super::test_block::Test;
use crate::logic::Offset;
use crate::shared::{
    ALLY_ADJACENT_COUNT, CAPTURE, CHECK_STATE, CONVERT, COPY_SOURCE, DEPENDS_ON, EXPLODE,
    LEG_EMPTY, MOVE, NOT_ATTACKED, PATH_NOT_ATTACKED, PATH_PIECE_COUNT, PIECE_AT, PIECE_FIRST_MOVE,
    PIECE_NOT_AT, SET_STATE,
};

/// What a move condition checks, read from its code when the blueprint is built.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConditionKind {
    DependsOn,
    CheckState,
    PieceFirstMove,
    NotAttacked,
    PathNotAttacked,
    PathPieceCount,
    PieceAt,
    PieceNotAt,
    AllyAdjacentCount,
    LegEmpty,
    /// A condition declared in the spec's `conditions` list, checked on a cell by its name.
    Named(String),
}

impl ConditionKind {
    /// The code the spec wrote for this condition.
    pub fn code(&self) -> &str {
        match self {
            Self::DependsOn => DEPENDS_ON,
            Self::CheckState => CHECK_STATE,
            Self::PieceFirstMove => PIECE_FIRST_MOVE,
            Self::NotAttacked => NOT_ATTACKED,
            Self::PathNotAttacked => PATH_NOT_ATTACKED,
            Self::PathPieceCount => PATH_PIECE_COUNT,
            Self::PieceAt => PIECE_AT,
            Self::PieceNotAt => PIECE_NOT_AT,
            Self::AllyAdjacentCount => ALLY_ADJACENT_COUNT,
            Self::LegEmpty => LEG_EMPTY,
            Self::Named(name) => name,
        }
    }
}

impl From<&str> for ConditionKind {
    fn from(value: &str) -> Self {
        match value {
            DEPENDS_ON => Self::DependsOn,
            CHECK_STATE => Self::CheckState,
            PIECE_FIRST_MOVE => Self::PieceFirstMove,
            NOT_ATTACKED => Self::NotAttacked,
            PATH_NOT_ATTACKED => Self::PathNotAttacked,
            PATH_PIECE_COUNT => Self::PathPieceCount,
            PIECE_AT => Self::PieceAt,
            PIECE_NOT_AT => Self::PieceNotAt,
            ALLY_ADJACENT_COUNT => Self::AllyAdjacentCount,
            LEG_EMPTY => Self::LegEmpty,
            name => Self::Named(name.to_string()),
        }
    }
}

/// What a side effect does, read from its action when the blueprint is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SideEffectKind {
    SetState,
    Capture,
    MovePiece,
    Convert,
    CopySource,
    Explode,
    /// An action the engine does not know; it does nothing.
    Unknown,
}

impl From<&str> for SideEffectKind {
    fn from(value: &str) -> Self {
        match value {
            SET_STATE => Self::SetState,
            CAPTURE => Self::Capture,
            MOVE => Self::MovePiece,
            CONVERT => Self::Convert,
            COPY_SOURCE => Self::CopySource,
            EXPLODE => Self::Explode,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Condition {
    pub kind: ConditionKind,

    // For DEPENDS_ON: the id of the move blueprint that must have produced a valid move.
    // None for all other condition types where move_id has no meaning.
    pub move_id: Option<u8>,

    // For CHECK_STATE: the name of the state flag to look for.
    pub state: Option<String>,

    // For CHECK_STATE and PIECE_FIRST_MOVE: relative offset from the source position,
    // pre-transformed per player at blueprint-build time (same convention as `step`).
    pub position: Option<HashMap<String, Offset>>,

    // For PIECE_AT / PIECE_NOT_AT / ALLY_ADJACENT_COUNT: piece code to check.
    pub piece_code: Option<String>,

    // For PATH_PIECE_COUNT / ALLY_ADJACENT_COUNT: inclusive lower bound (default 0).
    pub min: Option<u8>,

    // For PATH_PIECE_COUNT / ALLY_ADJACENT_COUNT: inclusive upper bound (default u8::MAX).
    pub max: Option<u8>,

    // For PIECE_FIRST_MOVE: pass when target square is off-board or empty.
    pub absent_ok: bool,

    // For PIECE_AT: player relationship to check ("SELF", "ENEMY", "ANY").
    pub player: Option<String>,
}

/// An entry of a move's or an action's conditions: a condition as written before tests, or a
/// test.
#[derive(Clone, Debug)]
pub enum Rule {
    Legacy(Condition),
    Test(Test),
}

/// When a repeating move stops: a condition named as before tests, or a test.
#[derive(Clone, Debug)]
pub enum Until {
    Named(String),
    Test(Test),
}

#[derive(Clone, Debug)]
pub struct MoveRepeat {
    pub until: Until,
    pub times: u8,
    pub loop_move: bool,
}

#[derive(Clone, Debug)]
pub struct Modifier {
    pub action: String,
    /// Judged with `START` the piece's square and `TARGET` the landing square; a legacy
    /// condition is read as a zone, by its name, on the landing square.
    pub conditions: Vec<Rule>,

    // Options for the modifier. I.e. pieces to transform into.
    pub options: Vec<String>,
}

/// Runtime representation of a side effect, with relative positions pre-transformed
/// per player at blueprint-build time.
#[derive(Clone, Debug)]
pub struct SideEffectBlueprint {
    /// What the side effect does ("SET_STATE", "CAPTURE", "MOVE" in the spec, and so on).
    pub kind: SideEffectKind,
    /// For SET_STATE: the state flag name to set on the moved piece.
    pub state: Option<String>,
    /// For SET_STATE: countdown duration. None = permanent (Blank flag).
    pub duration: Option<u8>,
    /// For MOVE side effect: source position (relative to the moving piece's source), per player.
    pub from: Option<HashMap<String, Offset>>,
    /// For MOVE side effect: destination position, per player.
    pub to: Option<HashMap<String, Offset>>,
    /// For CAPTURE side effect: position to clear (relative to the moving piece's source), per player.
    /// Also used by CONVERT as the position of the enemy to convert.
    pub target: Option<HashMap<String, Offset>>,
    /// For CONVERT: the piece code to place at the target square (belonging to the acting player).
    /// If absent, defaults to the acting piece's own code.
    pub piece: Option<String>,
    /// For EXPLODE: blast radius in squares (default 1).
    pub radius: Option<u8>,
    /// For EXPLODE: piece codes exempt from the blast on adjacent squares.
    pub exempt_codes: Vec<String>,
}

/// Runtime representation of a move action, bundling the action string with
/// optional action-level conditions (gates whether this action fires) and
/// action-level side effects (only applied when this action fires).
#[derive(Clone, Debug)]
pub struct ActionBlueprint {
    pub action: String,
    pub conditions: Vec<Rule>,
    pub side_effects: Vec<SideEffectBlueprint>,
}

/// A `MoveBlueprint` is a factory for a single move. The move could be repeatable (i.e. Rooks),
/// but it's a single, discrete type of logic.
///
/// Actions are indexed by the state of the position.
#[derive(Clone, Debug)]
pub struct MoveBlueprint {
    pub id: u8,
    /// When `true`, this blueprint contributes to the attack map but generates no legal moves.
    pub threat_only: bool,
    pub step: HashMap<String, Offset>, // player -> the edge label of the step

    /// When `true`, the step is a map's label: the move goes to every place it links to, each
    /// judged as a one-step move. Otherwise it is a grid vector, followed to one square.
    pub label_step: bool,

    // Players this move is available to. Empty means every player.
    pub players: Vec<String>,
    pub actions: HashMap<String, ActionBlueprint>,

    // Conditions that must be met for the move to be valid, judged in order.
    pub conditions: Vec<Rule>,

    // Modifiers for the move, containing the condition that must be met for
    // the modifier to be applied.
    pub modifiers: Vec<Modifier>,

    // Move-level side effects: applied whenever this move executes, regardless of which action fires.
    pub side_effects: Vec<SideEffectBlueprint>,

    // Number of times to repeat the move. `0u8` means repeat indefinitely.
    // Defaults to `1u8`.
    pub repeat_options: MoveRepeat,
}
