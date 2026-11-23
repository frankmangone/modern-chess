//! Actions and steps, compiled: built once per spec for each player, with every place's offset
//! turned for that player.

mod build;

use std::collections::HashMap;

use super::test_block::{Anchor, Filter, Place, Region, Test};

pub(crate) use build::steps_by_player;

/// A compiled step (see `DoSpec`).
#[derive(Clone, Debug)]
pub enum Step {
    Choose(Choice),
    Test(Test),
    Effect(EffectStep),
    If {
        test: Test,
        then: Vec<Step>,
        otherwise: Vec<Step>,
    },
}

/// A compiled choice (see `ChoiceSpec`).
#[derive(Clone, Debug)]
pub enum Choice {
    /// A place holding a piece that passes the filter.
    Piece(Filter),
    /// A place the piece at `START` reaches by its moves.
    Move,
    /// A place of the region.
    Place(Region),
    /// A piece in the mover's hand of one of these codes; empty means every code.
    Hand(Vec<String>),
    /// One of `options`, named `name`.
    Code { name: String, options: Vec<String> },
}

/// A compiled effect (see `EffectSpec`).
#[derive(Clone, Debug)]
pub enum EffectStep {
    Move {
        from: Place,
        to: Place,
    },
    Remove(Place),
    /// A new piece put on a place (the spec's `PLACE` effect).
    Put {
        at: Place,
        code: CodeRef,
    },
    SetOwner(Place),
    SetCode {
        at: Place,
        code: CodeRef,
    },
    SetFlag {
        at: Place,
        flag: String,
        turns: Option<u8>,
    },
    ForEach {
        region: Region,
        steps: Vec<Step>,
    },
    /// Adds to a counter of the mover, or of every player.
    Add {
        counter: String,
        by: i64,
        all: bool,
    },
    /// Sets a counter of the mover, or of every player, to 0.
    Reset {
        counter: String,
        all: bool,
    },
    EndTurn,
}

/// A piece code: written in the spec, or the answer to the `CODE` choice of that name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodeRef {
    Code(String),
    Chosen(String),
}

/// A compiled action: its steps for each player who may take it, by name.
#[derive(Clone, Debug)]
pub struct Action {
    pub name: String,
    /// The players who may take it; empty means every player.
    pub players: Vec<String>,
    pub steps: HashMap<String, Vec<Step>>,
}

/// Whether `steps` can remove more than their target: a `REMOVE` inside a `FOR_EACH`, or a
/// `REMOVE` at `START` with no offset.
pub(crate) fn removes_beyond_target(steps: &[Step]) -> bool {
    steps.iter().any(|step| removes_beyond(step, false))
}

/// Whether `step` removes more than its target; `in_for_each` when it runs inside a
/// `FOR_EACH`.
fn removes_beyond(step: &Step, in_for_each: bool) -> bool {
    match step {
        Step::Effect(EffectStep::Remove(at)) => {
            in_for_each || (at.anchor == Anchor::Start && at.offset.is_none())
        }
        Step::Effect(EffectStep::ForEach { steps, .. }) => {
            steps.iter().any(|step| removes_beyond(step, true))
        }
        Step::If {
            then, otherwise, ..
        } => then
            .iter()
            .chain(otherwise)
            .any(|step| removes_beyond(step, in_for_each)),
        Step::Choose(_) | Step::Test(_) | Step::Effect(_) => false,
    }
}
