mod build;
mod generate;

pub(crate) use build::direction_of;

use crate::shared::PlayerMap;

use crate::logic::rules::steps::Step;
use crate::logic::rules::test_block::Test;
use crate::logic::Offset;

#[derive(Clone, Debug)]
pub struct MoveRepeat {
    /// When the slide stops: judged on the next square (`TARGET`) before going on from it.
    pub until: Test,
    pub times: u8,
    pub loop_move: bool,
}

/// A `MoveBlueprint` is a factory for a single move. The move could be repeatable (i.e. Rooks),
/// but it's a single, discrete type of logic: what it does is its steps.
#[derive(Clone, Debug)]
pub struct MoveBlueprint {
    pub id: u8,
    /// When `true`, this blueprint contributes to the attack map but generates no legal moves.
    pub threat_only: bool,
    pub step: PlayerMap<Offset>, // player -> the edge label of the step

    /// When `true`, the step is a map's label: the move goes to every place it links to, each
    /// judged as a one-step move. Otherwise it is a grid vector, followed to one square.
    pub label_step: bool,

    // Players this move is available to. Empty means every player.
    pub players: Vec<String>,

    // Number of times to repeat the move. `0u8` means repeat indefinitely.
    // Defaults to `1u8`.
    pub repeat_options: MoveRepeat,

    /// What the move does, for each player it applies to.
    pub steps: PlayerMap<Vec<Step>>,

    /// Whether the move adds to the attack set: false when a test before its first effect
    /// requires `TARGET` to be empty.
    pub threatens: bool,
}
