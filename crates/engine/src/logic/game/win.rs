use std::sync::Arc;

use crate::logic::{Game, GamePhase};
use crate::shared::{Position, AREA_SCORE, CHECK_COUNT, CONNECTION, OPPONENT_BARE, PIECE_IN_ZONE};
use crate::specs::WinConditionSpec;

/// Which win condition a spec's entry is, read from its type when the game is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinConditionKind {
    PieceInZone,
    OpponentBare,
    CheckCount,
    AreaScore,
    Connection,
    /// A type the engine does not know; it never fires.
    Unknown,
}

/// A win condition of the game: its kind, and the spec entry holding its settings.
#[derive(Debug, Clone)]
pub struct WinCondition {
    pub kind: WinConditionKind,
    pub spec: WinConditionSpec,
}

impl From<WinConditionSpec> for WinCondition {
    fn from(spec: WinConditionSpec) -> Self {
        WinCondition {
            kind: WinConditionKind::from(spec.r#type.as_str()),
            spec,
        }
    }
}

impl From<&str> for WinConditionKind {
    fn from(value: &str) -> Self {
        match value {
            PIECE_IN_ZONE => Self::PieceInZone,
            OPPONENT_BARE => Self::OpponentBare,
            CHECK_COUNT => Self::CheckCount,
            AREA_SCORE => Self::AreaScore,
            CONNECTION => Self::Connection,
            _ => Self::Unknown,
        }
    }
}

impl Game {
    /// Eliminates `player` from the turn order, ending the game when one player remains.
    fn eliminate_player(&mut self, player: &str) {
        self.state.turn_order.retain(|p| p != player);
        // The cursor must stay inside the shrunken order even when the game ends here, or the
        // next look at whose turn it is reads past the end.
        if !self.state.turn_order.is_empty() {
            self.state.current_turn %= self.state.turn_order.len() as u8;
        }
        if self.state.turn_order.len() <= 1 {
            let winner = self.state.turn_order.first().cloned();
            self.state.phase = GamePhase::GameOver { winner };
            self.state.ways = Some(Vec::new());
        } else {
            let ways = self.compute_ways();
            self.start_choosing(ways);
        }
    }

    /// Determines whether the game is over once a turn has passed, and otherwise starts the
    /// choices of the player to move with their ways. A player with no way has no legal move.
    ///
    /// For N-player games: a checkmated player is eliminated from the turn order rather than
    /// immediately ending the game. The game ends only when one player remains.
    pub fn check_game_over(&mut self) {
        self.state.ways = Some(Vec::new());
        if self.check_win_conditions() {
            return;
        }
        if self.check_draws() {
            return;
        }
        let ways = self.compute_ways();
        if ways.is_empty() {
            let is_checkmate = self.rules.stalemate_loses || self.leader_in_check();
            if is_checkmate {
                // Checkmate (or stalemate-loses): eliminate the current player.
                let eliminated = self.current_player_ref().to_string();
                self.eliminate_player(&eliminated);
            } else {
                // Stalemate: draw.
                self.state.phase = GamePhase::GameOver { winner: None };
            }
        } else {
            self.start_choosing(ways);
        }
    }

    /// Evaluates all configured win conditions after a move.
    /// Returns `true` and sets `GamePhase::GameOver` if a win is detected.
    fn check_win_conditions(&mut self) -> bool {
        let prev_player = self.previous_player();
        let rules = Arc::clone(&self.rules);

        for WinCondition { kind, spec: wc } in &rules.win_conditions {
            match kind {
                WinConditionKind::PieceInZone => {
                    let (Some(piece_code), Some(zone)) = (&wc.piece, &wc.zone) else {
                        continue;
                    };
                    let positions: Vec<Position> = self
                        .state
                        .pieces
                        .iter()
                        .filter(|(_, p)| p.player == prev_player && &p.code == piece_code)
                        .map(|(pos, _)| pos.clone())
                        .collect();
                    let won = positions
                        .iter()
                        .any(|pos| self.in_zone(pos, zone, &prev_player));
                    if won {
                        self.state.phase = GamePhase::GameOver {
                            winner: Some(prev_player),
                        };
                        return true;
                    }
                }

                WinConditionKind::OpponentBare => {
                    let won = self
                        .rules
                        .players
                        .iter()
                        .filter(|p| p.as_str() != prev_player.as_str())
                        .all(|opp| {
                            self.state
                                .pieces
                                .values()
                                .filter(|p| p.player == *opp)
                                .all(|p| wc.exempt.contains(&p.code))
                        });
                    if won {
                        self.state.phase = GamePhase::GameOver {
                            winner: Some(prev_player),
                        };
                        return true;
                    }
                }

                WinConditionKind::CheckCount => {
                    let threshold = wc.threshold.unwrap_or(3);
                    // If the current player (who is now about to move) is in check,
                    // the previous player (who just moved) delivered that check.
                    if self.leader_in_check() {
                        let count = self
                            .state
                            .check_counts
                            .entry(prev_player.clone())
                            .or_insert(0);
                        *count += 1;
                        if *count >= threshold {
                            self.state.phase = GamePhase::GameOver {
                                winner: Some(prev_player),
                            };
                            return true;
                        }
                    }
                }

                WinConditionKind::AreaScore => {
                    if self.check_area_score(wc) {
                        return true;
                    }
                }

                WinConditionKind::Connection => {
                    if self.connects(&prev_player, wc) {
                        self.state.phase = GamePhase::GameOver {
                            winner: Some(prev_player),
                        };
                        return true;
                    }
                }

                WinConditionKind::Unknown => {}
            }
        }

        false
    }
}
