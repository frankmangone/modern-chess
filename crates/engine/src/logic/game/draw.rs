use std::collections::HashMap;

use crate::logic::{Game, GamePhase, Piece, PieceState};
use crate::shared::Position;

impl Game {
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
        let mut pieces: Vec<_> = pieces.iter().collect();
        pieces.sort_by(|a, b| a.0.cmp(b.0));

        let pieces_str: String = pieces
            .iter()
            .map(|(pos, p)| {
                let pos_s = pos
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
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
        if let Some(threshold) = self.repetition_count {
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
        if let Some(halfmoves) = self.fifty_move_halfmoves {
            let h = &self.state.history;
            let recent = h
                .len()
                .checked_sub(halfmoves as usize)
                .and_then(|from| h.get(from..));
            if let Some(recent) = recent {
                let no_reset = recent.iter().all(|r| {
                    r.action != "CAPTURE" && !self.fifty_move_pawn_codes.contains(&r.piece_code)
                });
                if no_reset {
                    self.state.phase = GamePhase::GameOver { winner: None };
                    return true;
                }
            }
        }

        // Insufficient material: draw when every player's piece set matches a configured entry.
        if !self.insufficient_material.is_empty() {
            let all_insufficient = self.players.iter().all(|player| {
                let mut player_pieces: Vec<String> = self
                    .state
                    .pieces
                    .values()
                    .filter(|p| &p.player == player)
                    .map(|p| p.code.clone())
                    .collect();
                player_pieces.sort();
                self.insufficient_material
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
}
