//! Building steps and actions from their specs.
//!
//! The spec's checker refuses unknown names and places that no earlier choice sets before a
//! game is built, so what is compiled here is already known to be well placed.

use std::collections::HashMap;

use super::{Action, Choice, CodeRef, EffectStep, Step};
use crate::logic::blueprint::test_block::{filter, Builder};
use crate::logic::Labels;
use crate::specs::game::{ActionSpec, ChoiceSpec, CodeSpec, DoSpec, EffectSpec, IfSpec};
use crate::specs::{PlayerSpec, StepSpec};

impl Step {
    /// Compiles `spec` for every player of `players`, as `Test::build` does. `step` is the
    /// move's step when the step belongs to a move, which places a `TARGET` one step from
    /// `START` when a `PATH`'s direction is worked out.
    pub fn build(
        spec: &DoSpec,
        players: &[PlayerSpec],
        step: Option<&StepSpec>,
        labels: &mut Labels,
    ) -> Step {
        Builder {
            players,
            step,
            labels,
        }
        .step(spec)
    }
}

impl Action {
    /// Compiles `spec` for each player of `players` who may take it.
    pub fn build(spec: &ActionSpec, players: &[PlayerSpec], labels: &mut Labels) -> Action {
        Action {
            name: spec.name.clone(),
            players: spec.players.clone(),
            steps: steps_by_player(&spec.r#do, players, &spec.players, None, labels),
        }
    }
}

/// `specs` compiled for each player of `players` that `allowed` names (every player when it is
/// empty), each with only that player's offsets.
pub(crate) fn steps_by_player(
    specs: &[DoSpec],
    players: &[PlayerSpec],
    allowed: &[String],
    step: Option<&StepSpec>,
    labels: &mut Labels,
) -> HashMap<String, Vec<Step>> {
    players
        .iter()
        .filter(|player| allowed.is_empty() || allowed.contains(&player.name))
        .map(|player| {
            let steps = specs
                .iter()
                .map(|spec| Step::build(spec, std::slice::from_ref(player), step, labels))
                .collect();
            (player.name.clone(), steps)
        })
        .collect()
}

impl Builder<'_> {
    fn step(&mut self, spec: &DoSpec) -> Step {
        match spec {
            DoSpec::Choose(choice) => Step::Choose(self.choice(choice)),
            DoSpec::Test(test) => Step::Test(self.test(test)),
            DoSpec::Effect(effect) => Step::Effect(self.effect(effect)),
            DoSpec::If(IfSpec { r#if, then, r#else }) => Step::If {
                test: self.test(r#if),
                then: self.steps(then),
                otherwise: self.steps(r#else),
            },
        }
    }

    fn steps(&mut self, specs: &[DoSpec]) -> Vec<Step> {
        specs.iter().map(|spec| self.step(spec)).collect()
    }

    fn choice(&mut self, spec: &ChoiceSpec) -> Choice {
        match spec {
            ChoiceSpec::Piece { r#where, .. } => Choice::Piece(filter(r#where)),
            ChoiceSpec::Move { .. } => Choice::Move,
            ChoiceSpec::Place { r#in, .. } => Choice::Place(self.region(r#in)),
            ChoiceSpec::Hand { r#where, .. } => Choice::Hand(
                r#where
                    .as_ref()
                    .map(|filter| filter.codes.clone())
                    .unwrap_or_default(),
            ),
            ChoiceSpec::Code { r#as, options } => Choice::Code {
                name: r#as.clone(),
                options: options.clone(),
            },
        }
    }

    fn effect(&mut self, spec: &EffectSpec) -> EffectStep {
        match spec {
            EffectSpec::Move { from, to } => EffectStep::Move {
                from: self.place(from),
                to: self.place(to),
            },
            EffectSpec::Remove { at } => EffectStep::Remove(self.place(at)),
            EffectSpec::Place { at, code } => EffectStep::Put {
                at: self.place(at),
                code: code_ref(code),
            },
            EffectSpec::SetOwner { at } => EffectStep::SetOwner(self.place(at)),
            EffectSpec::SetCode { at, code } => EffectStep::SetCode {
                at: self.place(at),
                code: code_ref(code),
            },
            EffectSpec::SetFlag { at, flag, turns } => EffectStep::SetFlag {
                at: self.place(at),
                flag: flag.clone(),
                turns: *turns,
            },
            EffectSpec::ForEach { r#in, r#do } => EffectStep::ForEach {
                region: self.region(r#in),
                steps: self.steps(r#do),
            },
            EffectSpec::EndTurn {} => EffectStep::EndTurn,
        }
    }
}

fn code_ref(spec: &CodeSpec) -> CodeRef {
    match spec {
        CodeSpec::Code(code) => CodeRef::Code(code.clone()),
        CodeSpec::Chosen { chosen } => CodeRef::Chosen(chosen.clone()),
    }
}
