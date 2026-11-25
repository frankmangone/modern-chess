//! Building a test from its spec, with every offset and direction turned for each player.
//!
//! The spec's checker refuses unknown names before a game is built, so a name that is not
//! one of the known values is read here as the default.

use super::{Attacker, ByPlayer, CountExpr, Filter, Owner, Region, Test, Through};
use crate::logic::pieces::moves::direction_of;
use crate::logic::places::holders::bag_position;
use crate::logic::places::place::{Anchor, Place};
use crate::logic::{LabelId, Labels};
use crate::shared::{
    apply_direction, ExtendedPosition, Position, Symbol, ADJACENT, ALLY, ENEMY, ORTHOGONAL,
};
use crate::specs::game::test_block::{
    AroundOf, FilterSpec, PlaceSpec, RegionSpec, TestSpec, EACH, FROM, LAST, MOVER, START, TO,
};
use crate::specs::game::CountSpec;
use crate::specs::{PlayerSpec, StepSpec};

impl Test {
    /// Compiles `spec` for every player of `players`. `step` is the move's step, which places a
    /// `TARGET` one step from `START` when a `PATH`'s direction is worked out.
    pub fn build(
        spec: &TestSpec,
        players: &[PlayerSpec],
        step: Option<&StepSpec>,
        labels: &mut Labels,
    ) -> Test {
        Builder {
            players,
            everyone: players,
            step,
            labels,
        }
        .test(spec)
    }
}

/// Compiles the blocks of a spec, for the players `players`: tests here, and the places and
/// regions of steps (see `Step::build`).
pub(crate) struct Builder<'a> {
    pub(crate) players: &'a [PlayerSpec],
    /// Every player of the game, which `players` may be one of (steps are built for one player
    /// at a time; `ENEMY` names another).
    pub(crate) everyone: &'a [PlayerSpec],
    pub(crate) step: Option<&'a StepSpec>,
    pub(crate) labels: &'a mut Labels,
}

impl Builder<'_> {
    pub(crate) fn test(&mut self, spec: &TestSpec) -> Test {
        match spec {
            TestSpec::All { of } => Test::All(of.iter().map(|test| self.test(test)).collect()),
            TestSpec::Any { of } => Test::Any(of.iter().map(|test| self.test(test)).collect()),
            TestSpec::Not { of } => Test::Not(Box::new(self.test(of))),
            TestSpec::Count {
                r#in,
                r#where,
                min,
                max,
            } => Test::Count {
                region: self.region(r#in),
                filter: r#where.as_ref().map(filter).unwrap_or_default(),
                min: min.unwrap_or(1),
                max: max.unwrap_or(u16::MAX),
            },
            TestSpec::Attacked {
                r#in,
                by,
                keeping,
                moves,
            } => Test::Attacked {
                region: self.region(r#in),
                by: match by.as_deref() {
                    Some(MOVER) => Attacker::Mover,
                    _ => Attacker::Enemy,
                },
                keeping: keeping.as_ref().map(filter),
                moves: moves.unwrap_or(1),
            },
            TestSpec::InZone { at, zone } => Test::InZone {
                at: self.place(at),
                zone: zone.clone(),
            },
            TestSpec::Removed { r#where } => Test::Removed(filter(r#where)),
            TestSpec::Repeats { times } => Test::Repeats {
                times: times.unwrap_or(1),
            },
            TestSpec::Moved { r#where } => Test::Moved(filter(r#where)),
            TestSpec::SomePlace { r#in, that } => Test::Some {
                region: self.region(r#in),
                that: Box::new(self.test(that)),
            },
            TestSpec::Same { a, b } => Test::Same {
                a: self.place(a),
                b: self.place(b),
            },
            TestSpec::Stuck {} => Test::Stuck,
            TestSpec::Passes { in_a_row } => Test::Passes(usize::from(*in_a_row)),
            TestSpec::Player { is } => Test::Player(is.clone()),
            TestSpec::Counter { name, min, max } => Test::Counter {
                name: name.clone(),
                min: min.unwrap_or(0),
                max: max.unwrap_or(i64::MAX),
            },
        }
    }

    pub(crate) fn region(&mut self, spec: &RegionSpec) -> Region {
        match spec {
            RegionSpec::At { at } => Region::At(self.place(at)),
            RegionSpec::Path { from, to, ends } => Region::Path {
                unit: self.unit_between(from, to),
                from: self.place(from),
                to: self.place(to),
                from_end: ends.iter().any(|end| end == FROM),
                to_end: ends.iter().any(|end| end == TO),
            },
            RegionSpec::Around { of, through } => {
                let through = self.through(through.as_deref());
                match of {
                    AroundOf::Place(place) => Region::Around {
                        of: self.place(place),
                        through,
                    },
                    AroundOf::Region(region) => Region::AroundRegion {
                        of: Box::new(self.region(region)),
                        through,
                    },
                }
            }
            RegionSpec::Group {
                of,
                through,
                r#where,
            } => Region::Group {
                of: self.place(of),
                through: self.through(through.as_deref()),
                filter: r#where.as_ref().map(filter).unwrap_or_default(),
            },
            RegionSpec::Line { through, along } => Region::Line {
                through: self.place(through),
                along: self
                    .players
                    .iter()
                    .map(|player| {
                        (
                            player.name.clone(),
                            apply_direction(direction_of(player), along),
                        )
                    })
                    .collect(),
            },
            RegionSpec::Follow {
                from,
                steps,
                through,
                skipping,
            } => Region::Follow {
                from: self.place(from),
                label: self
                    .labels
                    .named(through.as_deref().unwrap_or("next"))
                    .label,
                steps: self.count_expr(Some(steps)),
                skipping: skipping.clone(),
            },
            RegionSpec::Board { r#where } => Region::Board {
                filter: r#where.as_ref().map(filter),
            },
        }
    }

    /// How many pieces a count says: one when it is not written.
    pub(crate) fn count_expr(&mut self, count: Option<&CountSpec>) -> CountExpr {
        match count {
            None => CountExpr::Number(1),
            Some(CountSpec::Number(number)) => CountExpr::Number(*number),
            Some(CountSpec::All) => CountExpr::All,
            Some(CountSpec::Of(of)) => CountExpr::Of {
                region: Box::new(self.region(&of.r#in)),
                filter: of.r#where.as_ref().map(filter).unwrap_or_default(),
            },
        }
    }

    /// The neighbourhood written `through`: `adjacent` when it is not written.
    fn through(&mut self, through: Option<&str>) -> Through {
        match through {
            None | Some(ADJACENT) => Through::Adjacent,
            Some(ORTHOGONAL) => Through::Orthogonal,
            Some(label) => Through::Linked(self.labels.named(label).label),
        }
    }

    pub(crate) fn place(&mut self, spec: &PlaceSpec) -> Place {
        if let Some(bag) = &spec.bag {
            return Place {
                anchor: Anchor::Fixed,
                offset: None,
                fixed: Some(self.bag_for_each_player(bag, spec.of.as_deref())),
            };
        }
        let anchor = match spec.place.as_str() {
            START => Anchor::Start,
            EACH => Anchor::Each,
            LAST => Anchor::Last,
            _ => Anchor::Target,
        };
        Place {
            anchor,
            offset: spec.offset.as_ref().map(|offset| self.turned(offset)),
            fixed: None,
        }
    }

    /// The place of the bag `bag` for each player: `of` is MOVER (the player's own), ENEMY (the
    /// other player's), a player's name, or not written for a shared bag.
    pub(crate) fn bag_for_each_player(&self, bag: &str, of: Option<&str>) -> ByPlayer<Position> {
        self.players
            .iter()
            .map(|player| {
                let owner: Option<&str> = match of {
                    None => None,
                    Some("MOVER") => Some(player.name.as_str()),
                    Some("ENEMY") => self
                        .everyone
                        .iter()
                        .map(|other| other.name.as_str())
                        .find(|other| *other != player.name),
                    Some(name) => Some(name),
                };
                (player.name.clone(), bag_position(bag, owner))
            })
            .collect()
    }

    /// Each player's label for a vector of the spec.
    fn turned(&mut self, vector: &ExtendedPosition) -> ByPlayer<LabelId> {
        self.players
            .iter()
            .map(|player| {
                let offset = self.labels.for_player(direction_of(player), vector);
                (player.name.clone(), offset.label)
            })
            .collect()
    }

    /// Each player's unit step from `from` towards `to`: the unit of `to` − `from`, where a
    /// `TARGET` lies one step from `START`. `None` when the two are at the same spot, when one
    /// is `START` and the other `TARGET` while the step is not a vector, or when only one is
    /// `EACH`, which lies nowhere fixed.
    fn unit_between(&mut self, from: &PlaceSpec, to: &PlaceSpec) -> Option<ByPlayer<LabelId>> {
        let between_anchors: ExtendedPosition = if from.place == to.place {
            Vec::new()
        } else if from.place == EACH || to.place == EACH {
            return None;
        } else {
            let Some(StepSpec::Vector(step)) = self.step else {
                return None;
            };
            if from.place == START {
                step.clone()
            } else {
                step.iter().map(|value| value.saturating_neg()).collect()
            }
        };
        let no_offset = Vec::new();
        let from_offset = from.offset.as_ref().unwrap_or(&no_offset);
        let to_offset = to.offset.as_ref().unwrap_or(&no_offset);
        let axes = between_anchors
            .len()
            .max(from_offset.len())
            .max(to_offset.len());
        let at = |vector: &ExtendedPosition, axis: usize| vector.get(axis).copied().unwrap_or(0);
        let difference: ExtendedPosition = (0..axes)
            .map(|axis| {
                at(&between_anchors, axis)
                    .saturating_add(at(to_offset, axis))
                    .saturating_sub(at(from_offset, axis))
            })
            .collect();
        if difference.iter().all(|&value| value == 0) {
            return None;
        }
        Some(
            self.players
                .iter()
                .map(|player| {
                    let offset = self.labels.for_player(direction_of(player), &difference);
                    (player.name.clone(), offset.unit)
                })
                .collect(),
        )
    }
}

pub(crate) fn filter(spec: &FilterSpec) -> Filter {
    let owner = match spec.owner.as_deref() {
        Some(ALLY) => Owner::Ally,
        Some(ENEMY) => Owner::Enemy,
        _ => Owner::Any,
    };
    Filter {
        owner,
        codes: spec.codes.iter().map(|code| Symbol::new(code)).collect(),
        except: spec.except.iter().map(|code| Symbol::new(code)).collect(),
        flag: spec.flag.as_deref().map(Symbol::new),
        moved: spec.moved,
        empty: spec.empty,
    }
}
