// Basic states.
pub const ENEMY: &str = "ENEMY";
pub const ALLY: &str = "ALLY";

// Phase 17.A — win condition type strings.
pub const PIECE_IN_ZONE: &str = "PIECE_IN_ZONE";
pub const OPPONENT_BARE: &str = "OPPONENT_BARE";
pub const CHECK_COUNT: &str = "CHECK_COUNT";
/// Once the game has been ended by passing, the player whose pieces (and surrounded empty
/// points, when `territory` is set) cover more of the board wins.
pub const AREA_SCORE: &str = "AREA_SCORE";
/// The player who moved wins when one chain of their pieces, joined through a neighbourhood,
/// touches enough of a list of zones.
pub const CONNECTION: &str = "CONNECTION";

/// The neighbourhood of the cells one step along an axis (on a hex board: the six around).
pub const ORTHOGONAL: &str = "orthogonal";
/// The neighbourhood of every cell around a cell (on a hex board: the same six).
pub const ADJACENT: &str = "adjacent";
