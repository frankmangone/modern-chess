//! Building a move blueprint from its spec.

use std::collections::HashMap;

use super::{
    ActionBlueprint, Condition, ConditionKind, Modifier, MoveBlueprint, MoveRepeat, Rule,
    SideEffectBlueprint, SideEffectKind, Until,
};
use crate::logic::blueprint::test_block::Test;
use crate::logic::{Labels, Offset};
use crate::shared::{Direction, ExtendedPosition};
use crate::specs::game::piece::UntilSpec;
use crate::specs::game::test_block::{PlaceSpec, RegionSpec, TARGET};
use crate::specs::game::{ConditionEntry, TestSpec};
use crate::specs::{MoveSpec, PlayerSpec, StepSpec};

/// The direction of a player who has none: a player on a map, where no vector is followed.
static NO_DIRECTION: Direction = Vec::new();

/// A player's direction, or none.
pub(crate) fn direction_of(player: &PlayerSpec) -> &Direction {
    player.direction.as_ref().unwrap_or(&NO_DIRECTION)
}

impl MoveBlueprint {
    pub fn from_spec(spec: MoveSpec, players_spec: Vec<PlayerSpec>, labels: &mut Labels) -> Self {
        // Helper: each player's edge label for a relative position of the spec.
        fn transform_pos(
            raw: Option<ExtendedPosition>,
            players: &[PlayerSpec],
            labels: &mut Labels,
        ) -> Option<HashMap<String, Offset>> {
            raw.map(|relative| {
                players
                    .iter()
                    .map(|p| {
                        (
                            p.name.clone(),
                            labels.for_player(direction_of(p), &relative),
                        )
                    })
                    .collect()
            })
        }

        // Helper: convert a conditions list into rules, transforming any relative positions
        // per player. `step` is the move's, which a test's `TARGET` is read against.
        fn build_conditions(
            entries: &[ConditionEntry],
            players: &[PlayerSpec],
            step: &StepSpec,
            labels: &mut Labels,
        ) -> Vec<Rule> {
            entries
                .iter()
                .map(|entry| match entry {
                    ConditionEntry::Test(test) => {
                        Rule::Test(Test::build(test, players, Some(step), labels))
                    }
                    ConditionEntry::Legacy(c) => {
                        let position = transform_pos(c.position.clone(), players, labels);
                        Rule::Legacy(Condition {
                            kind: ConditionKind::from(c.condition.as_str()),
                            move_id: c.move_id,
                            state: c.state.clone(),
                            position,
                            piece_code: c.piece.clone(),
                            min: c.min,
                            max: c.max,
                            absent_ok: c.absent_ok,
                            player: c.player.clone(),
                        })
                    }
                })
                .collect()
        }

        // Build actions map: state -> ActionBlueprint.
        let mut actions = HashMap::new();
        for action_spec in spec.actions {
            let action_conditions =
                build_conditions(&action_spec.conditions, &players_spec, &spec.step, labels);
            let action_side_effects: Vec<SideEffectBlueprint> = action_spec
                .side_effects
                .iter()
                .map(|se| SideEffectBlueprint {
                    kind: SideEffectKind::from(se.action.as_str()),
                    state: se.state.clone(),
                    duration: se.duration,
                    from: transform_pos(se.from.clone(), &players_spec, labels),
                    to: transform_pos(se.to.clone(), &players_spec, labels),
                    target: transform_pos(se.target.clone(), &players_spec, labels),
                    piece: se.piece.clone(),
                    radius: se.radius,
                    exempt_codes: se.exempt_codes.clone(),
                })
                .collect();
            actions.insert(
                action_spec.state,
                ActionBlueprint {
                    action: action_spec.action,
                    conditions: action_conditions,
                    side_effects: action_side_effects,
                },
            );
        }

        // Transform each piece's canonical step by the player's direction matrix so that
        // "forward" in the spec maps to the correct board direction for every player. A map's
        // label is the same for every player.
        let mut step: HashMap<String, Offset> = HashMap::new();
        for player_spec in &players_spec {
            let player_step = match &spec.step {
                StepSpec::Vector(vector) => labels.for_player(direction_of(player_spec), vector),
                StepSpec::Label(name) => labels.named(name),
            };
            step.insert(player_spec.name.clone(), player_step);
        }
        let label_step = matches!(spec.step, StepSpec::Label(_));

        // Process repeat information. A repeat stops by default once the next square is
        // occupied.
        let (until, times, loop_move) = match spec.repeat {
            Some(repeat) => (repeat.until, repeat.times.unwrap_or(1u8), repeat.loop_move),
            None => (None, 1u8, false),
        };
        let until = match until {
            Some(UntilSpec::Legacy(name)) => Until::Named(name),
            Some(UntilSpec::Test(test)) => {
                Until::Test(Test::build(&test, &players_spec, Some(&spec.step), labels))
            }
            None => Until::Test(Test::build(
                &next_square_occupied(),
                &players_spec,
                Some(&spec.step),
                labels,
            )),
        };

        // Process move-level conditions.
        let conditions = build_conditions(&spec.conditions, &players_spec, &spec.step, labels);

        // Process modifiers.
        let modifiers = spec
            .modifiers
            .iter()
            .map(|m| Modifier {
                action: m.action.clone(),
                conditions: build_conditions(&m.conditions, &players_spec, &spec.step, labels),
                options: m.options.clone(),
            })
            .collect();

        // Process move-level side effects.
        let side_effects: Vec<SideEffectBlueprint> = spec
            .side_effects
            .iter()
            .map(|se| SideEffectBlueprint {
                kind: SideEffectKind::from(se.action.as_str()),
                state: se.state.clone(),
                duration: se.duration,
                from: transform_pos(se.from.clone(), &players_spec, labels),
                to: transform_pos(se.to.clone(), &players_spec, labels),
                target: transform_pos(se.target.clone(), &players_spec, labels),
                piece: se.piece.clone(),
                radius: se.radius,
                exempt_codes: se.exempt_codes.clone(),
            })
            .collect();

        MoveBlueprint {
            id: spec.id,
            threat_only: spec.threat_only,
            step,
            label_step,
            players: spec.players,
            actions,
            conditions,
            modifiers,
            side_effects,
            repeat_options: MoveRepeat {
                until,
                times,
                loop_move,
            },
        }
    }
}

/// A repeat's `until` when none is written: some piece stands on the next square.
fn next_square_occupied() -> TestSpec {
    TestSpec::Count {
        r#in: RegionSpec::At {
            at: PlaceSpec {
                place: TARGET.to_string(),
                offset: None,
            },
        },
        r#where: None,
        min: None,
        max: None,
    }
}
