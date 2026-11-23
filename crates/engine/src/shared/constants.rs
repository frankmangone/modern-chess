/// The most steps a `FOLLOW` takes. A board has far fewer places; a count above this is a
/// mistake, and walking it would exhaust memory or time.
pub const MAX_FOLLOW_STEPS: u32 = 100_000;

// Basic states.
pub const ENEMY: &str = "ENEMY";
pub const ALLY: &str = "ALLY";

/// The neighbourhood of the cells one step along an axis (on a hex board: the six around).
pub const ORTHOGONAL: &str = "orthogonal";
/// The neighbourhood of every cell around a cell (on a hex board: the same six).
pub const ADJACENT: &str = "adjacent";
