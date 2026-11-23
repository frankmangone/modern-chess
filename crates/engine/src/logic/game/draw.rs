use crate::logic::PiecesView;

use crate::logic::{Game, NodeId, PieceState};
use crate::shared::{Position, Symbol};

impl Game {
    /// Whether the position of `pieces` is on record at least `times` times. Inside a way
    /// (`settled` false) it is the position the way leaves, with the turn handed to the next
    /// player, which is not on record yet; after a turn (`settled`) it is the position as it
    /// stands, already on record once. The starting position is not recorded, so it never
    /// counts.
    pub(crate) fn position_recurs(
        &self,
        pieces: PiecesView<'_>,
        times: u16,
        settled: bool,
    ) -> bool {
        if self.state.position_hashes.is_empty() {
            return false;
        }
        let key = if settled {
            self.position_key_for(pieces, self.current_player_ref(), false)
        } else {
            // The turn passing ticks every piece's timed flags before the position is
            // recorded, and a piece in the hand that a test sees on the empty name is on no
            // place of the board.
            self.position_key_for(pieces, self.player_after_current(), true)
        };
        let seen = self
            .state
            .position_hashes
            .iter()
            .filter(|recorded| **recorded == key)
            .count();
        seen >= usize::from(times)
    }

    /// Builds a deterministic string key encoding the full position:
    /// active player + all pieces sorted by position, with their code, player,
    /// the has-moved flag (only where a rule reads `moved`), and state flags. Used for repetition detection.
    pub(crate) fn position_key(&self) -> String {
        self.position_key_for(
            PiecesView::of(&self.state.pieces),
            self.current_player_ref(),
            false,
        )
    }

    /// Like `position_key`, for any arrangement of pieces and any player to move. Used to ask
    /// whether a move would recreate a position without playing it. With `ticked` the key is of
    /// the position after the turn passes: timed flags counted down (and gone at 0) and the
    /// hand's place left out.
    pub(crate) fn position_key_for(
        &self,
        pieces: PiecesView<'_>,
        to_move: &str,
        ticked: bool,
    ) -> String {
        // Board order, which for a grid is the order of the coordinates. A position that is
        // not on the board (none should be) comes last.
        let nowhere = ticked.then(Position::default);
        let mut pieces: Vec<_> = pieces
            .iter()
            .filter(|(pos, _)| nowhere.as_ref() != Some(*pos))
            .collect();
        pieces.sort_by_cached_key(|(pos, _)| (self.board.node(pos).unwrap_or(NodeId::MAX), *pos));

        let pieces_str: String = pieces
            .iter()
            .map(|(pos, p)| {
                let pos_s = pos.as_str();
                let mut flags: Vec<(&Symbol, String)> = p
                    .state
                    .iter()
                    .filter_map(|(k, v)| {
                        let shown = match v {
                            PieceState::Uint(0) if ticked => return None,
                            PieceState::Uint(n) if ticked => format!("U{}", n - 1),
                            PieceState::Uint(n) => format!("U{n}"),
                            PieceState::Blank => "B".to_string(),
                            PieceState::String(s) => format!("S{s}"),
                        };
                        Some((k, shown))
                    })
                    .collect();
                flags.sort_by(|a, b| a.0.cmp(b.0));
                let flags_s = flags
                    .iter()
                    .map(|(k, v_s)| format!("{k}={v_s}"))
                    .collect::<Vec<_>>()
                    .join(";");
                // The has-moved flag is part of the position only where a rule reads it.
                let mv = if self.rules.moved_matters {
                    (p.total_moves > 0) as u8
                } else {
                    0
                };
                format!(
                    "[{pos_s}:{piece}:{player}:{mv}:{flags_s}]",
                    piece = p.code,
                    player = p.player,
                )
            })
            .collect();

        format!("{}|{}", to_move, pieces_str)
    }

    /// How many turns in a row, counting back from the latest, were passes (ways that changed
    /// nothing).
    pub fn consecutive_passes(&self) -> usize {
        self.state
            .history
            .iter()
            .rev()
            .take_while(|record| record.passed)
            .count()
    }
}
