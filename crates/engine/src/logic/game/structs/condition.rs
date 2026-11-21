use std::collections::{HashMap, HashSet};

use crate::shared::{POSITION, STATE};

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
