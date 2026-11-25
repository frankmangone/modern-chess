//! Actions and steps, compiled: built once per spec for each player, with every place's offset
//! turned for that player.

mod build;

use crate::shared::PlayerMap;

use super::test_block::{CountExpr, Filter, Region, Test};
use crate::logic::places::place::{Anchor, Place};
use crate::shared::Symbol;

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
    /// A piece held in the bag `bag` of one of these codes; empty means every code.
    Held { bag: Place, codes: Vec<String> },
    /// One of `options`, named `name`.
    Code { name: String, options: Vec<String> },
}

/// How many pieces an effect moves, removes or puts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Count {
    Number(u32),
    All,
}

/// A compiled effect (see `EffectSpec`).
#[derive(Clone, Debug)]
pub enum EffectStep {
    Move {
        from: Place,
        to: Place,
        /// Where the piece removed from `to` goes, if it goes anywhere: a bag.
        removed_to: Option<Place>,
        /// Which pieces move, from a holder.
        what: Option<Filter>,
        /// How many move, from a holder.
        count: CountExpr,
    },
    Remove {
        at: Place,
        /// Where the piece removed goes, if it goes anywhere: a bag.
        removed_to: Option<Place>,
        /// Which pieces are removed, from a holder.
        what: Option<Filter>,
        /// How many are removed, from a holder.
        count: CountExpr,
    },
    /// New pieces put on a place (the spec's `PLACE` effect).
    Put {
        at: Place,
        code: CodeRef,
        /// How many, on a holder.
        count: CountExpr,
    },
    SetOwner(Place),
    SetCode {
        at: Place,
        code: CodeRef,
    },
    SetFlag {
        at: Place,
        flag: Symbol,
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
    /// The turn is not handed on: the same player moves again in the stage.
    Stage(Symbol),
    /// Other players answer, one after another, in a stage (see `EffectSpec::Ask`).
    Ask(AskStep),
    /// Dice rolled into a bag: each die lands on one of the faces, a set of pieces.
    Roll {
        at: Place,
        dice: CountExpr,
        faces: Vec<Vec<(Symbol, u32)>>,
    },
    /// An ordered bag put in a random order.
    Shuffle(Place),
}

/// A piece code: written in the spec, or the answer to the `CODE` choice of that name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodeRef {
    Code(String),
    Chosen(String),
}

/// Whom an `ASK` asks (see `AskPlayers`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AskWho {
    Others,
    All,
    Mover,
    Named(Vec<String>),
}

/// A compiled `ASK`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskStep {
    pub who: AskWho,
    /// The stage the asked seats answer in.
    pub stage: Symbol,
    /// The stage the asker moves again in, once every seat has answered.
    pub then: Option<Symbol>,
}

/// A compiled action: its steps for each player who may take it, by name.
#[derive(Clone, Debug)]
pub struct Action {
    pub name: String,
    /// The stage it belongs to, if it does: it is offered only there.
    pub stage: Option<Symbol>,
    /// The players who may take it; empty means every player.
    pub players: Vec<String>,
    pub steps: PlayerMap<Vec<Step>>,
}

/// Whether `steps` roll or shuffle, inside an `if` or a `FOR_EACH` too.
pub(crate) fn uses_chance(steps: &[Step]) -> bool {
    steps.iter().any(|step| match step {
        Step::Effect(EffectStep::Roll { .. } | EffectStep::Shuffle(_)) => true,
        Step::Effect(EffectStep::ForEach { steps, .. }) => uses_chance(steps),
        Step::If {
            then, otherwise, ..
        } => uses_chance(then) || uses_chance(otherwise),
        Step::Choose(_) | Step::Test(_) | Step::Effect(_) => false,
    })
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
        Step::Effect(EffectStep::Remove { at, .. }) => {
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
