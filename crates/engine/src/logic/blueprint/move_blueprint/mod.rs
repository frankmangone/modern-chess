mod build;
mod generate;

pub(crate) use build::direction_of;

use std::collections::HashMap;

use super::steps::Step;
use super::test_block::Test;
use crate::logic::Offset;
use crate::shared::{CAPTURE, CONVERT, COPY_SOURCE, EXPLODE, MOVE, SET_STATE};

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
pub struct MoveRepeat {
    /// When the slide stops: judged on the next square (`TARGET`) before going on from it.
    pub until: Test,
    pub times: u8,
    pub loop_move: bool,
}

#[derive(Clone, Debug)]
pub struct Modifier {
    pub action: String,
    /// Judged with `START` the piece's square and `TARGET` the landing square.
    pub conditions: Vec<Test>,

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

/// Runtime representation of a move's old occupancy-keyed action (replaced by the move's
/// steps), bundling the action string with
/// optional action-level conditions (gates whether this action fires) and
/// action-level side effects (only applied when this action fires).
#[derive(Clone, Debug)]
pub struct LegacyActionBlueprint {
    pub action: String,
    pub conditions: Vec<Test>,
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
    pub actions: HashMap<String, LegacyActionBlueprint>,

    // Conditions that must be met for the move to be valid, judged in order.
    pub conditions: Vec<Test>,

    // Modifiers for the move, containing the condition that must be met for
    // the modifier to be applied.
    pub modifiers: Vec<Modifier>,

    // Move-level side effects: applied whenever this move executes, regardless of which action fires.
    pub side_effects: Vec<SideEffectBlueprint>,

    // Number of times to repeat the move. `0u8` means repeat indefinitely.
    // Defaults to `1u8`.
    pub repeat_options: MoveRepeat,

    /// What the move does, for each player it applies to: `None` on a move written in the old
    /// forms.
    pub steps: Option<HashMap<String, Vec<Step>>>,

    /// Whether the move adds to the attack set: false when a test before its first effect
    /// requires `TARGET` to be empty (in the old forms, when it has no `ENEMY` action).
    pub threatens: bool,
}
