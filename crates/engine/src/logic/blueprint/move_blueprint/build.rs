//! Building a move blueprint from its spec.

use std::collections::HashMap;

use super::{
    ActionBlueprint, Condition, ConditionKind, Modifier, MoveBlueprint, MoveRepeat,
    SideEffectBlueprint, SideEffectKind,
};
use crate::logic::{Labels, Offset};
use crate::shared::{ExtendedPosition, NOT_EMPTY};
use crate::specs::{MoveSpec, PlayerSpec};

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
                    .map(|p| (p.name.clone(), labels.for_player(&p.direction, &relative)))
                    .collect()
            })
        }

        // Helper: convert a ConditionSpec slice into Condition structs, transforming
        // any relative positions per player.
        fn build_conditions(
            specs: &[crate::specs::game::piece::condition::ConditionSpec],
            players: &[PlayerSpec],
            labels: &mut Labels,
        ) -> Vec<Condition> {
            specs
                .iter()
                .map(|c| {
                    let position = transform_pos(c.position.clone(), players, labels);
                    Condition {
                        kind: ConditionKind::from(c.condition.as_str()),
                        move_id: c.move_id,
                        state: c.state.clone(),
                        position,
                        piece_code: c.piece.clone(),
                        min: c.min,
                        max: c.max,
                        absent_ok: c.absent_ok,
                        player: c.player.clone(),
                    }
                })
                .collect()
        }

        // Build actions map: state -> ActionBlueprint.
        let mut actions = HashMap::new();
        for action_spec in spec.actions {
            let action_conditions =
                build_conditions(&action_spec.conditions, &players_spec, labels);
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
        // "forward" in the spec maps to the correct board direction for every player.
        let mut step: HashMap<String, Offset> = HashMap::new();
        for player_spec in &players_spec {
            let player_step = labels.for_player(&player_spec.direction, &spec.step);
            step.insert(player_spec.name.clone(), player_step);
        }

        // Process repeat information.
        let (until, times, loop_move) = match spec.repeat {
            Some(repeat) => (
                repeat.until.unwrap_or_else(|| NOT_EMPTY.to_string()),
                repeat.times.unwrap_or(1u8),
                repeat.loop_move,
            ),
            None => (NOT_EMPTY.to_string(), 1u8, false),
        };

        // Process move-level conditions.
        let conditions = build_conditions(&spec.conditions, &players_spec, labels);

        // Process modifiers.
        let modifiers = spec
            .modifiers
            .iter()
            .map(|m| Modifier {
                action: m.action.clone(),
                conditions: m
                    .conditions
                    .iter()
                    .map(|c| Condition {
                        kind: ConditionKind::from(c.condition.as_str()),
                        move_id: None, // modifier conditions never use DEPENDS_ON
                        state: c.state.clone(),
                        position: None,
                        piece_code: c.piece.clone(),
                        min: c.min,
                        max: c.max,
                        absent_ok: c.absent_ok,
                        player: None,
                    })
                    .collect(),
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
