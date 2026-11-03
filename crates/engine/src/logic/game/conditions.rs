use crate::logic::Game;
use crate::shared::{into_string, Position, ALLY, EMPTY, ENEMY, NOT_EMPTY, POSITION, STATE};

impl Game {
    pub fn check_position_condition(&self, position: &Position, condition: &String) -> bool {
        self.check_position_condition_str(position, condition.as_str())
    }

    pub(crate) fn check_position_condition_str(
        &self,
        position: &Position,
        condition: &str,
    ) -> bool {
        self.check_position_condition_for_player_str(position, condition, self.current_player_ref())
    }

    /// Like `check_position_condition` but uses an explicit player instead of the current player.
    /// Used by win-condition checks that need to evaluate zones from the mover's perspective.
    pub fn check_position_condition_for_player(
        &self,
        position: &Position,
        condition: &String,
        player: &str,
    ) -> bool {
        self.check_position_condition_for_player_str(position, condition.as_str(), player)
    }

    pub(crate) fn check_position_condition_for_player_str(
        &self,
        position: &Position,
        condition: &str,
        player: &str,
    ) -> bool {
        let maybe_piece = self.piece_at_position_ref(position);

        // Built-in occupancy conditions.
        match maybe_piece {
            Some(p) => {
                if condition == NOT_EMPTY {
                    return true;
                }
                if condition == ALLY && p.player == player {
                    return true;
                }
                if condition == ENEMY && p.player != player {
                    return true;
                }
            }
            None => {
                if condition == EMPTY {
                    return true;
                }
            }
        };

        // Custom conditions (POSITION / STATE).
        let Some(condition_def) = self.conditions.get(condition) else {
            return false;
        };

        let condition_value = match condition_def.r#type.as_str() {
            POSITION => &into_string(position),
            STATE => condition,
            _ => return false,
        };

        let Some(player_set) = condition_def.check.get(player) else {
            return false;
        };
        player_set.contains(condition_value)
    }
}
