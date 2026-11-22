//! Conditions declared in a spec's `conditions` list, and the checks they make on a cell.

use std::collections::{HashMap, HashSet};

use crate::logic::Game;
use crate::shared::{Position, ALLY, EMPTY, ENEMY, NOT_EMPTY, POSITION, STATE};

/// What a declared condition checks on a cell, read from its type when the game is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellConditionKind {
    /// The cell is one of the listed positions.
    Position,
    /// The condition's own name is in the list.
    State,
    /// A type the engine does not know; it never holds.
    Unknown,
}

impl From<&str> for CellConditionKind {
    fn from(value: &str) -> Self {
        match value {
            POSITION => Self::Position,
            STATE => Self::State,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConditionDef {
    /// What the condition checks.
    pub kind: CellConditionKind,

    /// The actual condition, with information about what to check.
    pub check: HashMap<String, HashSet<String>>,
}

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
        let Some(condition_def) = self.rules.conditions.get(condition) else {
            return false;
        };

        let condition_value = match condition_def.kind {
            CellConditionKind::Position => position.as_str(),
            CellConditionKind::State => condition,
            CellConditionKind::Unknown => return false,
        };

        let Some(player_set) = condition_def.check.get(player) else {
            return false;
        };
        player_set.contains(condition_value)
    }
}
