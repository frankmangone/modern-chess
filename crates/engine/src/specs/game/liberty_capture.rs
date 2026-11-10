use serde::{Deserialize, Serialize};

/// Captures pieces that have been shut in, as in Go.
///
/// Pieces of the listed codes that belong to the same player and touch along a row or a column
/// form a group. A group's liberties are the empty points next to it. After every move or
/// placement, each group of the player who did *not* just play that has no liberties left is
/// removed. A group of the player who just played that has none is a suicide, which is refused
/// unless `allow_suicide` is set (in which case the group is removed too).
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LibertyCaptureSpec {
    /// The piece codes that form groups. Any other piece is neither captured nor counted as
    /// a liberty, and blocks the way like the edge of the board.
    pub pieces: Vec<String>,

    /// Whether a placement that leaves the player's own group without liberties (and captures
    /// nothing) is allowed. Defaults to `false`.
    #[serde(default)]
    pub allow_suicide: bool,
}
