use std::collections::{BTreeMap, HashSet};

use crate::logic::{Game, GamePhase};
use crate::shared::{Position, AREA_SCORE};
use crate::specs::WinConditionSpec;

impl Game {
    /// The first area-score win condition of the game, if it has one.
    fn area_score_rule(&self) -> Option<&WinConditionSpec> {
        self.win_conditions
            .iter()
            .find(|wc| wc.r#type == AREA_SCORE)
    }

    /// Where every player stands under the game's area-score rule: the pieces they have on the
    /// board, plus the empty regions only they touch when the rule counts territory, plus their
    /// bonus. `None` when the game has no such rule.
    pub fn area_scores(&self) -> Option<BTreeMap<String, f64>> {
        self.area_score_rule().map(|rule| self.scores_under(rule))
    }

    fn scores_under(&self, rule: &WinConditionSpec) -> BTreeMap<String, f64> {
        let counts = |code: &str| rule.pieces.is_empty() || rule.pieces.iter().any(|c| c == code);

        let mut scores: BTreeMap<String, f64> = self
            .players
            .iter()
            .map(|player| {
                (
                    player.clone(),
                    rule.bonus.get(player).copied().unwrap_or(0.0),
                )
            })
            .collect();

        for piece in self.state.pieces.values().filter(|p| counts(&p.code)) {
            *scores.entry(piece.player.clone()).or_insert(0.0) += 1.0;
        }

        if rule.territory {
            for (owner, size) in self.empty_regions(&counts) {
                *scores.entry(owner).or_insert(0.0) += size as f64;
            }
        }
        scores
    }

    /// The empty regions that touch the counted pieces of exactly one player, as (owner, size).
    /// A region that touches nobody, or more than one player, belongs to no one. Pieces that are
    /// not counted are not part of any region and do not make one touch a player.
    fn empty_regions(&self, counts: &dyn Fn(&str) -> bool) -> Vec<(String, usize)> {
        let mut owned = Vec::new();
        let mut visited: HashSet<Position> = HashSet::new();

        for start in self.board.all_positions() {
            if self.state.pieces.contains_key(&start) || visited.contains(&start) {
                continue;
            }
            let mut size = 0;
            let mut touching: HashSet<&str> = HashSet::new();
            let mut stack = vec![start];
            while let Some(position) = stack.pop() {
                if !visited.insert(position.clone()) {
                    continue;
                }
                size += 1;
                for next in self.orthogonal_neighbours(&position) {
                    match self.state.pieces.get(&next) {
                        None => stack.push(next),
                        Some(piece) if counts(&piece.code) => {
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

    /// Whether the game has just been ended by passing, as `rule` defines it.
    fn passed_out(&self, rule: &WinConditionSpec) -> bool {
        let needed = rule
            .after_passes
            .map_or(self.players.len(), usize::from)
            .max(1);
        self.consecutive_passes() >= needed
    }

    /// Ends the game when enough passes in a row have happened, giving it to the player with the
    /// highest score, or to nobody when the best score is shared. Returns `true` when it ended.
    pub(crate) fn check_area_score(&mut self, rule: &WinConditionSpec) -> bool {
        if !self.passed_out(rule) {
            return false;
        }
        let scores = self.scores_under(rule);
        let best = scores.values().copied().fold(f64::NEG_INFINITY, f64::max);
        let mut leaders = scores.iter().filter(|(_, score)| **score == best);
        let winner = match (leaders.next(), leaders.next()) {
            (Some((player, _)), None) => Some(player.clone()),
            _ => None,
        };
        self.state.phase = GamePhase::GameOver { winner };
        true
    }
}
