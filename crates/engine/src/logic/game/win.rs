use crate::logic::{Game, GamePhase};
use crate::shared::{Position, CHECK_COUNT, OPPONENT_BARE, PIECE_IN_ZONE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WinConditionType {
    PieceInZone,
    OpponentBare,
    CheckCount,
    Unknown,
}

impl From<&str> for WinConditionType {
    fn from(value: &str) -> Self {
        match value {
            PIECE_IN_ZONE => Self::PieceInZone,
            OPPONENT_BARE => Self::OpponentBare,
            CHECK_COUNT => Self::CheckCount,
            _ => Self::Unknown,
        }
    }
}

impl Game {
    /// Eliminates `player` from the turn order, ending the game when one player remains.
    fn eliminate_player(&mut self, player: &str) {
        self.turn_order.retain(|p| p != player);
        if self.turn_order.len() <= 1 {
            let winner = self.turn_order.first().cloned();
            self.state.phase = GamePhase::GameOver { winner };
        } else {
            self.state.current_turn %= self.turn_order.len() as u8;
            self.state.phase = GamePhase::Idle;
        }
    }

    /// Determines whether the game is over after a move/transform and updates the phase.
    ///
    /// For N-player games: a checkmated player is eliminated from the turn order rather than
    /// immediately ending the game. The game ends only when one player remains.
    pub fn check_game_over(&mut self) {
        if self.check_win_conditions() {
            return;
        }
        if self.check_draws() {
            return;
        }
        if !self.any_legal_moves() {
            let is_checkmate = self.stalemate_loses || self.leader_in_check();
            if is_checkmate {
                // Checkmate (or stalemate-loses): eliminate the current player.
                let eliminated = self.current_player_ref().to_string();
                self.eliminate_player(&eliminated);
            } else {
                // Stalemate: draw.
                self.state.phase = GamePhase::GameOver { winner: None };
            }
        } else {
            self.state.phase = GamePhase::Idle;
        }
    }

    /// Evaluates all configured win conditions after a move.
    /// Returns `true` and sets `GamePhase::GameOver` if a win is detected.
    fn check_win_conditions(&mut self) -> bool {
        let prev_player = self.previous_player();
        let wcs = self.win_conditions.clone();

        for wc in &wcs {
            match WinConditionType::from(wc.r#type.as_str()) {
                WinConditionType::PieceInZone => {
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
                    let won = positions.iter().any(|pos| {
                        self.check_position_condition_for_player_str(pos, zone, &prev_player)
                    });
                    if won {
                        self.state.phase = GamePhase::GameOver {
                            winner: Some(prev_player),
                        };
                        return true;
                    }
                }

                WinConditionType::OpponentBare => {
                    let won = self
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

                WinConditionType::CheckCount => {
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

                WinConditionType::Unknown => {}
            }
        }

        false
    }
}
