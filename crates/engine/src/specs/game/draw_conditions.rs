use serde::{Deserialize, Serialize};

/// Spec-level draw condition configuration. All fields default to "disabled"
/// so games that don't declare draw conditions behave identically to before.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DrawConditionsSpec {
    /// Force a draw when the same position (board + active player + piece state)
    /// has been reached this many times. `null` / absent = disabled.
    /// Chess uses 3; Shogi sennichite uses 4.
    #[serde(default)]
    pub repetition_count: Option<u8>,

    /// Number of consecutive half-moves with no pawn push and no capture after
    /// which the game is a draw. `null` / absent = disabled. Chess uses 100.
    #[serde(default)]
    pub fifty_move_halfmoves: Option<u16>,

    /// Piece codes that, when moved (action = MOVE), reset the fifty-move counter.
    /// Typically `["PAWN"]`. Only meaningful when `fifty_move_halfmoves` is set.
    #[serde(default)]
    pub fifty_move_pawn_codes: Vec<String>,

    /// Piece-code multisets that indicate insufficient mating material for a single
    /// player. The game is drawn when every player's full piece set matches one of
    /// these entries (sorted multiset equality). Example chess entries:
    /// `["KING"]`, `["KING","BISHOP"]`, `["KING","KNIGHT"]`.
    #[serde(default)]
    pub insufficient_material: Vec<Vec<String>>,
}
