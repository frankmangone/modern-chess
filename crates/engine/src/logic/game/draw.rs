use std::collections::HashMap;

use crate::logic::{Game, GamePhase, NodeId, Piece, PieceState};
use crate::shared::Position;

impl Game {
    /// Whether the position of `pieces` is on record at least `times` times. Inside a way
    /// (`settled` false) it is the position the way leaves, with the turn handed to the next
    /// player, which is not on record yet; after a turn (`settled`) it is the position as it
    /// stands, already on record once. The starting position is not recorded, so it never
    /// counts.
    pub(crate) fn position_recurs(
        &self,
        pieces: &HashMap<Position, Piece>,
        times: u16,
        settled: bool,
    ) -> bool {
        if self.state.position_hashes.is_empty() {
            return false;
        }
        let key = if settled {
            self.position_key_for(pieces, self.current_player_ref())
        } else {
            let mut after = pieces.clone();
            // A piece in the hand that a test sees on the empty name is on no place of the board.
            after.remove(&Position::default());
            // The turn passing ticks every piece's timed flags before the position is recorded.
            for piece in after.values_mut() {
                piece.tick_state_flags();
            }
            self.position_key_for(&after, self.player_after_current())
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
    /// total_moves, and state flags. Used for repetition detection.
    pub(crate) fn position_key(&self) -> String {
        self.position_key_for(&self.state.pieces, self.current_player_ref())
    }

    /// Like `position_key`, for any arrangement of pieces and any player to move. Used to ask
    /// whether a move would recreate a position without playing it.
    pub(crate) fn position_key_for(
        &self,
        pieces: &HashMap<Position, Piece>,
        to_move: &str,
    ) -> String {
        // Board order, which for a grid is the order of the coordinates. A position that is
        // not on the board (none should be) comes last.
        let mut pieces: Vec<_> = pieces.iter().collect();
        pieces.sort_by_cached_key(|(pos, _)| (self.board.node(pos).unwrap_or(NodeId::MAX), *pos));

        let pieces_str: String = pieces
            .iter()
            .map(|(pos, p)| {
                let pos_s = pos.as_str();
                let mut flags: Vec<_> = p.state.iter().collect();
                flags.sort_by(|a, b| a.0.cmp(b.0));
                let flags_s = flags
                    .iter()
                    .map(|(k, v)| {
                        let v_s = match v {
                            PieceState::Blank => "B".to_string(),
                            PieceState::Uint(n) => format!("U{n}"),
                            PieceState::String(s) => format!("S{s}"),
                        };
                        format!("{k}={v_s}")
                    })
                    .collect::<Vec<_>>()
                    .join(";");
                format!(
                    "[{pos_s}:{piece}:{player}:{mv}:{flags_s}]",
                    piece = p.code,
                    player = p.player,
                    mv = (p.total_moves > 0) as u8,
                )
            })
            .collect();

        format!("{}|{}", to_move, pieces_str)
    }

    /// Records the current position and checks all configured draw conditions.
    /// Returns `true` and sets `GameOver { winner: None }` if a draw is detected.
    pub(crate) fn check_draws(&mut self) -> bool {
        let key = self.position_key();
        self.state.position_hashes.push(key.clone());

        // Repetition draw.
        if let Some(threshold) = self.rules.repetition_count {
            let count = self
                .state
                .position_hashes
                .iter()
                .filter(|h| *h == &key)
                .count();
            if count >= threshold as usize {
                self.state.phase = GamePhase::GameOver { winner: None };
                return true;
            }
        }

        // Fifty-move rule (N consecutive half-moves with no pawn push and no capture).
        if let Some(halfmoves) = self.rules.fifty_move_halfmoves {
            let h = &self.state.history;
            let recent = h
                .len()
                .checked_sub(halfmoves as usize)
                .and_then(|from| h.get(from..));
            if let Some(recent) = recent {
                let no_reset = recent.iter().all(|r| {
                    !r.captures && !self.rules.fifty_move_pawn_codes.contains(&r.piece_code)
                });
                if no_reset {
                    self.state.phase = GamePhase::GameOver { winner: None };
                    return true;
                }
            }
        }

        // Insufficient material: draw when every player's piece set matches a configured entry.
        if !self.rules.insufficient_material.is_empty() {
            let all_insufficient = self.rules.players.iter().all(|player| {
                let mut player_pieces: Vec<String> = self
                    .state
                    .pieces
                    .values()
                    .filter(|p| &p.player == player)
                    .map(|p| p.code.clone())
                    .collect();
                player_pieces.sort();
                self.rules
                    .insufficient_material
                    .iter()
                    .any(|entry| player_pieces == *entry)
            });
            if all_insufficient {
                self.state.phase = GamePhase::GameOver { winner: None };
                return true;
            }
        }

        false
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
