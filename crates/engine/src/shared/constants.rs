// Basic states.
pub const EMPTY: &str = "EMPTY";
pub const NOT_EMPTY: &str = "NOT_EMPTY";
pub const ENEMY: &str = "ENEMY";
pub const ALLY: &str = "ALLY";

// Basic actions.
pub const MOVE: &str = "MOVE";
pub const CAPTURE: &str = "CAPTURE";
pub const SET_STATE: &str = "SET_STATE";

// Basic conditions.
pub const DEPENDS_ON: &str = "DEPENDS_ON";
/// Check that the piece at a relative position carries a named state flag.
pub const CHECK_STATE: &str = "CHECK_STATE";
/// Check that the piece at a relative position has never moved (total_moves == 0).
/// `position: [0, 0]` checks the moving piece itself (replaces the old FIRST_MOVE shorthand).
/// When `absent_ok: true`, passes if the square is off-board or empty.
pub const PIECE_FIRST_MOVE: &str = "PIECE_FIRST_MOVE";
/// Check that a square is not in any opponent's attack set.
/// Default (no `position`): checks the move target. `position: [0, 0]`: checks the source
/// square (replaces old SOURCE_NOT_ATTACKED shorthand).
pub const NOT_ATTACKED: &str = "NOT_ATTACKED";
pub const PATH_NOT_ATTACKED: &str = "PATH_NOT_ATTACKED";

// Basic condition types.
pub const POSITION: &str = "POSITION";
pub const STATE: &str = "STATE";
pub const TRANSFORM: &str = "TRANSFORM";

// Drop-related actions/conditions.
pub const DROP: &str = "DROP";
pub const ALLY_ON_FILE: &str = "ALLY_ON_FILE";

// Phase 17.B — new move conditions.
/// Count pieces strictly between source and target; pass when count in [min, max].
pub const PATH_PIECE_COUNT: &str = "PATH_PIECE_COUNT";
/// Piece at relative position must have the specified code.
pub const PIECE_AT: &str = "PIECE_AT";
/// Piece at relative position must NOT have the specified code (or square is empty/off-board).
pub const PIECE_NOT_AT: &str = "PIECE_NOT_AT";
/// Count ally pieces in the 8 surrounding squares; pass when count in [min, max].
pub const ALLY_ADJACENT_COUNT: &str = "ALLY_ADJACENT_COUNT";

/// The square at the relative `position` offset from the source must be empty.
/// Used for horse-leg blocking: the first orthogonal step in the L-shape must be unoccupied.
/// `position` is written in neutral coordinates and rotated per-player at build time.
pub const LEG_EMPTY: &str = "LEG_EMPTY";

// Phase 17.A — win condition type strings.
pub const PIECE_IN_ZONE: &str = "PIECE_IN_ZONE";
pub const OPPONENT_BARE: &str = "OPPONENT_BARE";
pub const CHECK_COUNT: &str = "CHECK_COUNT";

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
