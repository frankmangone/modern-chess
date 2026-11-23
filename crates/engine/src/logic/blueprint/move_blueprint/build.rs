//! Building a move blueprint from its spec.

use crate::shared::PlayerMap;

use super::{MoveBlueprint, MoveRepeat};
use crate::logic::blueprint::steps::steps_by_player;
use crate::logic::blueprint::test_block::Test;
use crate::logic::{Labels, Offset};
use crate::shared::Direction;
use crate::specs::game::test_block::{PlaceSpec, RegionSpec, TARGET};
use crate::specs::game::{DoSpec, TestSpec};
use crate::specs::{MoveSpec, PlayerSpec, StepSpec};

/// The direction of a player who has none: a player on a map, where no vector is followed.
static NO_DIRECTION: Direction = Vec::new();

/// A player's direction, or none.
pub(crate) fn direction_of(player: &PlayerSpec) -> &Direction {
    player.direction.as_ref().unwrap_or(&NO_DIRECTION)
}

impl MoveBlueprint {
    pub fn from_spec(spec: MoveSpec, players_spec: Vec<PlayerSpec>, labels: &mut Labels) -> Self {
        // A move threatens unless it requires an empty target.
        let threatens = !requires_empty_target(&spec.r#do);
        let steps = steps_by_player(
            &spec.r#do,
            &players_spec,
            &spec.players,
            Some(&spec.step),
            labels,
        );

        // Transform each piece's canonical step by the player's direction matrix so that
        // "forward" in the spec maps to the correct board direction for every player. A map's
        // label is the same for every player.
        let mut step: PlayerMap<Offset> = PlayerMap::new();
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
        let until = Test::build(
            &until.unwrap_or_else(next_square_occupied),
            &players_spec,
            Some(&spec.step),
            labels,
        );

        MoveBlueprint {
            id: spec.id,
            threat_only: spec.threat_only,
            step,
            label_step,
            players: spec.players,
            repeat_options: MoveRepeat {
                until,
                times,
                loop_move,
            },
            steps,
            threatens,
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

/// Whether a test before the first effect of `steps` requires `TARGET` to be empty, written on
/// its own or in a top-level `ALL`.
fn requires_empty_target(steps: &[DoSpec]) -> bool {
    steps
        .iter()
        .take_while(|step| matches!(step, DoSpec::Choose(_) | DoSpec::Test(_)))
        .filter_map(|step| match step {
            DoSpec::Test(TestSpec::All { of }) => Some(of.as_slice()),
            DoSpec::Test(test) => Some(std::slice::from_ref(test)),
            _ => None,
        })
        .flatten()
        .any(empties_target)
}

/// Whether `test` holds only when `TARGET` is empty: `NOT(COUNT in AT(TARGET))`, or
/// `COUNT in AT(TARGET)` with `min: 0, max: 0`, each with no `where`.
fn empties_target(test: &TestSpec) -> bool {
    match test {
        TestSpec::Not { of } => matches!(
            &**of,
            TestSpec::Count { r#in, r#where: None, min: None | Some(1), max: None } if at_target(r#in)
        ),
        TestSpec::Count {
            r#in,
            r#where: None,
            min: Some(0),
            max: Some(0),
        } => at_target(r#in),
        _ => false,
    }
}

/// Whether `region` is the one place `TARGET`, with no offset.
fn at_target(region: &RegionSpec) -> bool {
    matches!(region, RegionSpec::At { at: PlaceSpec { place, offset: None } } if place == TARGET)
}
