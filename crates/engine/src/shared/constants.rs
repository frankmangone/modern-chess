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
pub const FIRST_MOVE: &str = "FIRST_MOVE";
pub const DEPENDS_ON: &str = "DEPENDS_ON";
/// Check that the piece at a relative position carries a named state flag.
pub const CHECK_STATE: &str = "CHECK_STATE";
/// Check that the piece at a relative position has never moved (total_moves == 0).
pub const PIECE_FIRST_MOVE: &str = "PIECE_FIRST_MOVE";
/// Alias kept for backward compatibility with chess.json.
pub const ROOK_FIRST_MOVE: &str = "ROOK_FIRST_MOVE";
/// Check that every square strictly between source and target is empty.
pub const PATH_EMPTY: &str = "PATH_EMPTY";

// Unimplemented conditions (Phase 4 / Phase 5).
pub const NOT_ATTACKED: &str = "NOT_ATTACKED";
pub const PATH_NOT_ATTACKED: &str = "PATH_NOT_ATTACKED";

// Basic condition types.
pub const POSITION: &str = "POSITION";
pub const STATE: &str = "STATE";
pub const TRANSFORM: &str = "TRANSFORM";
