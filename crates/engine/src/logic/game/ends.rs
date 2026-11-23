//! How a game ends: the end checks, judged in order after every turn, and the score.

use std::collections::{BTreeMap, HashSet};

use crate::logic::blueprint::test_block::{
    filter, judge, matches, AfterTurn, Filter, Test, TestContext,
};
use crate::logic::{Game, GamePhase, Labels, NodeId, Way};
use crate::shared::Position;
use crate::specs::game::{EndCheckSpec, OutcomeSpec, ScoreTermSpec, Viewpoint};
use crate::specs::PlayerSpec;

/// What follows when an end check holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Win,
    Lose,
    Draw,
    Highest,
}

/// A compiled end check (see `EndCheckSpec`).
#[derive(Clone, Debug)]
pub struct EndCheck {
    /// Judged from the point of view of the player about to play, not the one who just played.
    pub next: bool,
    pub test: Test,
    pub outcome: Outcome,
    /// Whether the test asks if the player to move has a way, so the ways must be worked out.
    pub reads_ways: bool,
}

impl EndCheck {
    pub fn build(spec: &EndCheckSpec, players: &[PlayerSpec], labels: &mut Labels) -> Self {
        let test = Test::build(&spec.when, players, None, labels);
        EndCheck {
            next: spec.viewpoint == Viewpoint::Next,
            reads_ways: reads_ways(&test),
            test,
            outcome: match spec.then {
                OutcomeSpec::Win {} => Outcome::Win,
                OutcomeSpec::Lose {} => Outcome::Lose,
                OutcomeSpec::Draw {} => Outcome::Draw,
                OutcomeSpec::Highest {} => Outcome::Highest,
            },
        }
    }
}

/// Whether judging `test` needs to know if the player to move has a way.
fn reads_ways(test: &Test) -> bool {
    match test {
        Test::Stuck => true,
        Test::All(tests) | Test::Any(tests) => tests.iter().any(reads_ways),
        Test::Not(test) => reads_ways(test),
        Test::Some { that, .. } => reads_ways(that),
        _ => false,
    }
}

/// A compiled score term (see `ScoreTermSpec`).
#[derive(Clone, Debug)]
pub enum ScoreTerm {
    Pieces(Filter),
    Territory(Filter),
    Counter(String),
    Points { value: f64, to: Vec<String> },
}

impl From<&ScoreTermSpec> for ScoreTerm {
    fn from(spec: &ScoreTermSpec) -> Self {
        match spec {
            ScoreTermSpec::Pieces { r#where } => {
                ScoreTerm::Pieces(r#where.as_ref().map(filter).unwrap_or_default())
            }
            ScoreTermSpec::Territory { r#where } => {
                ScoreTerm::Territory(r#where.as_ref().map(filter).unwrap_or_default())
            }
            ScoreTermSpec::Counter { name } => ScoreTerm::Counter(name.clone()),
            ScoreTermSpec::Points { value, to } => ScoreTerm::Points {
                value: *value,
                to: to.clone(),
            },
        }
    }
}

impl Game {
    /// Where every player stands under the game's `score`: the sum of its terms. `None` when
    /// the game has no score.
    pub fn scores(&self) -> Option<BTreeMap<String, f64>> {
        if self.rules.score.is_empty() {
            return None;
        }
        // Each territory term is worked out once for every player.
        let territories: Vec<Option<Vec<(String, usize)>>> = self
            .rules
            .score
            .iter()
            .map(|term| match term {
                ScoreTerm::Territory(filter) => Some(self.territory_regions(filter)),
                _ => None,
            })
            .collect();
        let scores = self
            .rules
            .players
            .iter()
            .map(|player| {
                let total = self
                    .rules
                    .score
                    .iter()
                    .zip(&territories)
                    .map(|(term, territory)| match term {
                        ScoreTerm::Pieces(filter) => self
                            .state
                            .pieces
                            .values()
                            .filter(|piece| {
                                piece.player == *player && matches(filter, piece, player)
                            })
                            .count() as f64,
                        ScoreTerm::Territory(_) => territory
                            .iter()
                            .flatten()
                            .filter(|(owner, _)| owner == player)
                            .map(|(_, size)| *size as f64)
                            .sum(),
                        ScoreTerm::Counter(name) => self.counter(player, name) as f64,
                        ScoreTerm::Points { value, to } => {
                            if to.contains(player) {
                                *value
                            } else {
                                0.0
                            }
                        }
                    })
                    .sum();
                (player.clone(), total)
            })
            .collect();
        Some(scores)
    }

    /// The empty regions that touch the pieces passing `counted` of exactly one player, as
    /// (owner, size). A region that touches nobody, or more than one player, belongs to no one.
    /// Pieces that do not pass are not part of any region and do not make one touch a player.
    fn territory_regions(&self, counted: &Filter) -> Vec<(String, usize)> {
        let mut owned = Vec::new();
        let mut visited: HashSet<NodeId> = HashSet::new();
        let board = &self.board;

        for start in board.positions() {
            let Some(start) = board.node(start) else {
                continue;
            };
            let named = |node: NodeId| board.name(node);
            if named(start).is_some_and(|name| self.state.pieces.contains_key(name))
                || visited.contains(&start)
            {
                continue;
            }
            let mut size = 0;
            let mut touching: HashSet<&str> = HashSet::new();
            let mut stack = vec![start];
            while let Some(node) = stack.pop() {
                if !visited.insert(node) {
                    continue;
                }
                size += 1;
                for next in board.orthogonal(node) {
                    let Some(name) = named(next) else {
                        continue;
                    };
                    match self.state.pieces.get(name) {
                        None => stack.push(next),
                        Some(piece) if matches(counted, piece, "") => {
                            touching.insert(piece.player.as_str());
                        }
                        Some(_) => {}
                    }
                }
            }
            let mut owners = touching.into_iter();
            if let (Some(owner), None) = (owners.next(), owners.next()) {
                owned.push((owner.to_string(), size));
            }
        }
        owned
    }

    /// Judges the end checks in order, for the turn that has just passed. The first that holds
    /// ends the game or removes a player, and `true` is returned. The ways of the player to
    /// move are worked out only if a check asks, and then once; they are handed back so the
    /// turn can start with them.
    pub(crate) fn check_end_checks(&mut self, ways: &mut Option<Vec<Way>>) -> bool {
        let rules = std::sync::Arc::clone(&self.rules);
        let nowhere = Position::default();
        for check in &rules.end_checks {
            let player = if check.next {
                self.current_player()
            } else {
                self.previous_player()
            };
            let after_turn = AfterTurn {
                stuck: if check.reads_ways {
                    ways.get_or_insert_with(|| self.compute_ways()).is_empty()
                } else {
                    false
                },
            };
            let holds = judge(
                &check.test,
                &TestContext {
                    game: self,
                    pieces: &self.state.pieces,
                    player: &player,
                    start: None,
                    target: &nowhere,
                    each: None,
                    removed: &[],
                    moved: &[],
                    after_turn: Some(after_turn),
                },
            );
            if holds {
                self.apply_outcome(check.outcome, &player);
                return true;
            }
        }
        false
    }

    /// Ends the game, or removes `player`, as `outcome` says.
    fn apply_outcome(&mut self, outcome: Outcome, player: &str) {
        match outcome {
            Outcome::Win => self.end_with(Some(player.to_string())),
            Outcome::Draw => self.end_with(None),
            Outcome::Lose => self.eliminate_player(player),
            Outcome::Highest => {
                let scores = self.scores().unwrap_or_default();
                let best = scores.values().copied().fold(f64::NEG_INFINITY, f64::max);
                let mut leaders = scores.iter().filter(|(_, score)| **score == best);
                let winner = match (leaders.next(), leaders.next()) {
                    (Some((player, _)), None) => Some(player.clone()),
                    _ => None,
                };
                self.end_with(winner);
            }
        }
    }

    fn end_with(&mut self, winner: Option<String>) {
        self.state.phase = GamePhase::GameOver { winner };
        self.state.ways = Some(Vec::new());
    }
}
