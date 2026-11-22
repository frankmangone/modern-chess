//! The CONNECTION win condition: one chain of a player's pieces, joined through a
//! neighbourhood, touching enough of a list of zones.

use std::collections::HashSet;

use crate::logic::{Game, NodeId};
use crate::shared::{Position, ADJACENT};
use crate::specs::WinConditionSpec;

impl Game {
    /// True when one chain of `player`'s pieces (of the rule's `pieces`, or all of them)
    /// touches at least `at_least` of the rule's zones, read with `player`'s cells.
    pub(crate) fn connects(&self, player: &str, rule: &WinConditionSpec) -> bool {
        let needed = rule.at_least.map_or(rule.zones.len(), usize::from).max(1);
        let mine: HashSet<NodeId> = self
            .state
            .pieces
            .iter()
            .filter(|(_, piece)| {
                piece.player == player
                    && (rule.pieces.is_empty() || rule.pieces.contains(&piece.code))
            })
            .filter_map(|(position, _)| self.board.node(position))
            .collect();
        let through_adjacent = rule.through.as_deref() == Some(ADJACENT);

        let mut seen: HashSet<NodeId> = HashSet::new();
        for &start in &mine {
            if !seen.insert(start) {
                continue;
            }
            let mut chain = vec![start];
            let mut frontier = vec![start];
            while let Some(node) = frontier.pop() {
                let around = if through_adjacent {
                    self.board.adjacent(node)
                } else {
                    self.board.orthogonal(node)
                };
                for next in around {
                    if mine.contains(&next) && seen.insert(next) {
                        chain.push(next);
                        frontier.push(next);
                    }
                }
            }
            if self.zones_touched(player, rule, &chain) >= needed {
                return true;
            }
        }
        false
    }

    /// How many of the rule's zones some node of `chain` lies in, for `player`.
    fn zones_touched(&self, player: &str, rule: &WinConditionSpec, chain: &[NodeId]) -> usize {
        let cells: Vec<&Position> = chain
            .iter()
            .filter_map(|&node| self.board.name(node))
            .collect();
        rule.zones
            .iter()
            .filter(|zone| cells.iter().any(|cell| self.in_zone(cell, zone, player)))
            .count()
    }
}
