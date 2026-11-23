use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::shared::Symbol;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Piece {
    pub code: Symbol,
    pub player: Symbol,
    pub total_moves: u16,
    pub state: HashMap<Symbol, PieceState>,
}

/// Value stored alongside a named state flag on a piece.
///
/// # Duration convention (`Uint`)
/// `PieceState::Uint(N)` acts as a countdown driven by `next_turn()`:
/// - Each call to `next_turn()` decrements `N` by 1.
/// - When `N` reaches **0** the flag is still visible for that turn, then removed
///   at the *following* `next_turn()` call.
///
/// In practice: `duration: 1` in a spec side-effect means "the flag will be
/// present for exactly **one** opponent turn after it is set."
///
/// # Permanent flags
/// `Blank` and `String` variants are never decremented and persist until
/// explicitly removed by a side-effect.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PieceState {
    Blank,
    Uint(u16),
    String(String),
}

impl Piece {
    pub fn new(code: impl Into<Symbol>, player: impl Into<Symbol>) -> Self {
        Piece {
            code: code.into(),
            player: player.into(),
            total_moves: 0u16,
            state: HashMap::new(),
        }
    }

    /// Ticks all duration-tracked (`Uint`) state flags by one step.
    /// Flags already at `0` are removed; flags at `N > 0` become `N - 1`.
    /// Called once per `next_turn()`.
    pub fn tick_state_flags(&mut self) {
        self.state.retain(|_, v| match v {
            PieceState::Uint(0) => false,
            PieceState::Uint(n) => {
                *n -= 1;
                true
            }
            _ => true,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_piece_with_flags_saves_and_loads_as_text() {
        let mut piece = Piece::new("PAWN", "WHITE");
        piece
            .state
            .insert(Symbol::new("EN_PASSANT"), PieceState::Uint(1));
        let json = serde_json::to_value(&piece).unwrap();
        assert_eq!(json["code"], "PAWN");
        assert_eq!(json["player"], "WHITE");
        assert!(json["state"]["EN_PASSANT"].is_object());
        let back: Piece = serde_json::from_value(json).unwrap();
        assert_eq!(back, piece);
    }
}
