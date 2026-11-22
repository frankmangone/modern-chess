// Basic states.
pub const EMPTY: &str = "EMPTY";
pub const ENEMY: &str = "ENEMY";
pub const ALLY: &str = "ALLY";

// Basic actions.
pub const MOVE: &str = "MOVE";
pub const CAPTURE: &str = "CAPTURE";
pub const SET_STATE: &str = "SET_STATE";
pub const TRANSFORM: &str = "TRANSFORM";

// Drop-related actions.
pub const DROP: &str = "DROP";

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

/// The action recorded in the history when a player gives up their turn.
pub const PASS: &str = "PASS";

// Phase 17.C — new side-effect actions.
/// Convert the enemy piece at a relative position into an ally piece of the specified code.
/// Fires only when an enemy piece occupies the target square; no-ops on empty/ally squares.
pub const CONVERT: &str = "CONVERT";
/// Place a copy of the acting piece back at its original source square (used for clone moves
/// where the source stone should remain in place while a new stone appears at the target).
pub const COPY_SOURCE: &str = "COPY_SOURCE";
/// Atomic-chess explosion: clear all pieces within `radius` squares of the landing square,
/// except pieces whose code appears in `exempt_codes` (e.g. pawns are exempt from adjacent blast).
/// The landing square (center) is always cleared regardless of exempt_codes.
pub const EXPLODE: &str = "EXPLODE";
