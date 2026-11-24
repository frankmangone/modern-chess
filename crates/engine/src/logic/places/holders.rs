//! Holders: places that keep counts of pieces instead of a piece. A bag (the hand among them) is
//! a holder off the board; a map place with a capacity above one is a holder on it.

use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use crate::logic::Game;
use crate::shared::{Position, Symbol};

/// Some pieces of one kind in a holder: who owns them (nobody, for the empty name), their code and
/// how many.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Held {
    pub owner: Symbol,
    pub code: Symbol,
    pub count: u32,
}

/// The place a bag is: `NAME@PLAYER` for a bag each player has, `NAME` for a shared one. Names of
/// bags and of the places of the board never meet (`@` is refused in both, and a bag may not
/// share a name with a place).
pub fn bag_position(name: &str, owner: Option<&str>) -> Position {
    match owner {
        Some(player) => Position::new(format!("{name}@{player}")),
        None => Position::new(name),
    }
}

/// What a holder keeps: counts per owner and code, in the order they first arrived.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Holder(Vec<Held>);

impl Holder {
    /// The counts, in the order they first arrived.
    pub fn entries(&self) -> &[Held] {
        &self.0
    }

    /// How many pieces it keeps, of every kind.
    pub fn total(&self) -> u32 {
        self.0
            .iter()
            .fold(0u32, |total, held| total.saturating_add(held.count))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many pieces of `code` it keeps, whoever owns them.
    pub fn count_of(&self, code: &str) -> u32 {
        self.0
            .iter()
            .filter(|held| held.code == *code)
            .fold(0u32, |total, held| total.saturating_add(held.count))
    }

    /// Adds `count` pieces of `code` owned by `owner`.
    pub fn add(&mut self, owner: Symbol, code: Symbol, count: u32) {
        if count == 0 {
            return;
        }
        match self
            .0
            .iter_mut()
            .find(|held| held.owner == owner && held.code == code)
        {
            Some(held) => held.count = held.count.saturating_add(count),
            None => self.0.push(Held { owner, code, count }),
        }
    }

    /// Takes `count` pieces of `code` out, from the first owner that arrived and on, and returns
    /// them as they were kept; `None`, taking nothing, when fewer are kept.
    pub fn take(&mut self, code: &str, count: u32) -> Option<Vec<Held>> {
        if self.count_of(code) < count {
            return None;
        }
        let mut left = count;
        let mut taken = Vec::new();
        for held in self.0.iter_mut().filter(|held| held.code == *code) {
            let part = held.count.min(left);
            if part > 0 {
                held.count -= part;
                left -= part;
                taken.push(Held {
                    owner: held.owner,
                    code: held.code,
                    count: part,
                });
            }
        }
        self.0.retain(|held| held.count > 0);
        Some(taken)
    }

    /// Adds (`delta` above 0) or takes (below 0) pieces of one owner and code; a take that finds
    /// fewer takes what is there.
    pub fn apply(&mut self, owner: Symbol, code: Symbol, delta: i32) {
        let count = delta.unsigned_abs();
        if delta >= 0 {
            self.add(owner, code, count);
            return;
        }
        if let Some(held) = self
            .0
            .iter_mut()
            .find(|held| held.owner == owner && held.code == code)
        {
            held.count = held.count.saturating_sub(count);
        }
        self.0.retain(|held| held.count > 0);
    }

    /// Puts `count` pieces on top of an ordered pile (the first entry is the top), joining the
    /// run of the same owner and code that is already on top.
    pub fn push_top(&mut self, owner: Symbol, code: Symbol, count: u32) {
        if count == 0 {
            return;
        }
        match self.0.first_mut() {
            Some(top) if top.owner == owner && top.code == code => {
                top.count = top.count.saturating_add(count);
            }
            _ => self.0.insert(0, Held { owner, code, count }),
        }
    }

    /// Takes the `count` pieces on top of an ordered pile, top first, and returns them as the
    /// runs they were kept in; `None`, taking nothing, when fewer are kept.
    pub fn pop_top(&mut self, count: u32) -> Option<Vec<Held>> {
        if self.total() < count {
            return None;
        }
        let mut left = count;
        let mut taken = Vec::new();
        for held in self.0.iter_mut() {
            if left == 0 {
                break;
            }
            let part = held.count.min(left);
            held.count -= part;
            left -= part;
            taken.push(Held {
                owner: held.owner,
                code: held.code,
                count: part,
            });
        }
        self.0.retain(|held| held.count > 0);
        Some(taken)
    }

    /// The same pieces in a random order: `pick(n)` is a number in `0..n`, a Fisher-Yates
    /// shuffle that asks for one number per piece but the last. `None` when the pile is
    /// larger than `MAX_SHUFFLED`.
    pub fn shuffled(&self, mut pick: impl FnMut(u32) -> u32) -> Option<Holder> {
        if self.total() > MAX_SHUFFLED {
            return None;
        }
        let mut pieces: Vec<(Symbol, Symbol)> = Vec::new();
        for held in &self.0 {
            for _ in 0..held.count {
                pieces.push((held.owner, held.code));
            }
        }
        for last in (1..pieces.len()).rev() {
            let chosen = pick(u32::try_from(last + 1).ok()?) as usize;
            pieces.swap(last, chosen.min(last));
        }
        let mut shuffled = Holder::default();
        for (owner, code) in pieces.into_iter().rev() {
            shuffled.push_top(owner, code, 1);
        }
        Some(shuffled)
    }

    /// The codes kept, each with how many, in the order they first arrived.
    pub fn codes(&self) -> Vec<(Symbol, u32)> {
        let mut codes: Vec<(Symbol, u32)> = Vec::new();
        for held in &self.0 {
            match codes.iter_mut().find(|(code, _)| *code == held.code) {
                Some((_, count)) => *count = count.saturating_add(held.count),
                None => codes.push((held.code, held.count)),
            }
        }
        codes
    }
}

/// What a bag keeps: its name, whose it is (`None` for a shared bag), and how many of each
/// piece code it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BagContents {
    pub name: String,
    pub owner: Option<String>,
    pub pieces: std::collections::BTreeMap<String, u32>,
}

/// The most pieces a pile can have and still be shuffled: the shuffle works on the pieces one by
/// one. A bigger pile makes no way that shuffles it.
pub const MAX_SHUFFLED: u32 = 10_000;

/// What a holder is, besides what it keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HolderInfo {
    /// Whether it keeps its pieces in order, top first: a pile (see `Holder::push_top`).
    pub ordered: bool,
    /// The most pieces it holds; `None` for no limit.
    pub capacity: Option<u32>,
    /// Whose pieces it keeps.
    pub owner: HolderOwner,
}

/// Whose pieces a holder keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HolderOwner {
    /// A player's bag: every piece that enters it is the player's.
    Player(Symbol),
    /// A shared bag: its pieces are nobody's.
    Shared,
    /// A place of the board: its pieces keep their owner.
    Place,
}

impl Game {
    /// What every bag keeps, sorted by name and then owner. A bag a player has none of is still
    /// listed, empty.
    pub fn bags(&self) -> Vec<BagContents> {
        let mut bags: Vec<BagContents> = self
            .rules
            .holders
            .iter()
            .filter(|(_, info)| info.owner != HolderOwner::Place)
            .map(|(at, _)| {
                let (name, owner) = match at.as_str().split_once('@') {
                    Some((name, owner)) => (name, Some(owner.to_string())),
                    None => (at.as_str(), None),
                };
                let pieces = self
                    .state
                    .holders
                    .get(at)
                    .map(|held| {
                        held.codes()
                            .into_iter()
                            .map(|(code, count)| (code.to_string(), count))
                            .collect()
                    })
                    .unwrap_or_default();
                BagContents {
                    name: name.to_string(),
                    owner,
                    pieces,
                }
            })
            .collect();
        bags.sort_by(|a, b| (&a.name, &a.owner).cmp(&(&b.name, &b.owner)));
        bags
    }

    /// The pieces in every player's hand (the bags `HAND`): player → code → count.
    pub fn hand(&self) -> HashMap<String, HashMap<String, u32>> {
        self.rules
            .players
            .iter()
            .filter_map(|player| {
                let held = self
                    .state
                    .holders
                    .get(&bag_position("HAND", Some(player)))?;
                let counts = held
                    .codes()
                    .into_iter()
                    .map(|(code, count)| (code.to_string(), count))
                    .collect();
                Some((player.clone(), counts))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn symbol(name: &str) -> Symbol {
        Symbol::new(name)
    }

    #[test]
    fn counts_add_up_per_owner_and_code() {
        let mut holder = Holder::default();
        holder.add(symbol("WHITE"), symbol("PAWN"), 2);
        holder.add(symbol("WHITE"), symbol("PAWN"), 1);
        holder.add(symbol(""), symbol("PAWN"), 4);
        holder.add(symbol("WHITE"), symbol("ROOK"), 0);
        assert_eq!(holder.entries().len(), 2);
        assert_eq!(holder.count_of("PAWN"), 7);
        assert_eq!(holder.count_of("ROOK"), 0);
        assert_eq!(holder.total(), 7);
        assert_eq!(holder.codes(), [(symbol("PAWN"), 7)]);
    }

    #[test]
    fn taking_goes_from_the_first_owner_and_never_takes_too_many() {
        let mut holder = Holder::default();
        holder.add(symbol("WHITE"), symbol("PAWN"), 2);
        holder.add(symbol("BLACK"), symbol("PAWN"), 3);
        assert!(holder.take("PAWN", 6).is_none());
        assert_eq!(holder.total(), 5, "a refused take takes nothing");
        let taken = holder.take("PAWN", 3).expect("enough kept");
        assert_eq!(
            taken,
            [
                Held {
                    owner: symbol("WHITE"),
                    code: symbol("PAWN"),
                    count: 2
                },
                Held {
                    owner: symbol("BLACK"),
                    code: symbol("PAWN"),
                    count: 1
                },
            ]
        );
        assert_eq!(holder.entries().len(), 1, "an emptied count is dropped");
        assert_eq!(holder.take("PAWN", 2).map(|taken| taken.len()), Some(1));
        assert!(holder.is_empty());
    }

    #[test]
    fn a_pile_takes_and_gives_at_the_top() {
        let mut pile = Holder::default();
        pile.push_top(symbol(""), symbol("A"), 2);
        pile.push_top(symbol(""), symbol("B"), 1);
        pile.push_top(symbol(""), symbol("B"), 1);
        // Top first: B, B, A, A.
        assert_eq!(pile.entries().len(), 2);
        assert_eq!(pile.total(), 4);
        assert!(pile.pop_top(5).is_none());
        assert_eq!(pile.total(), 4, "a refused take takes nothing");
        let taken = pile.pop_top(3).expect("enough kept");
        assert_eq!(
            taken
                .iter()
                .map(|held| (held.code.to_string(), held.count))
                .collect::<Vec<_>>(),
            [("B".to_string(), 2), ("A".to_string(), 1)]
        );
        assert_eq!(pile.total(), 1);
        assert_eq!(pile.count_of("A"), 1);
    }

    #[test]
    fn a_shuffle_keeps_the_pieces_and_follows_the_numbers() {
        let mut pile = Holder::default();
        for (code, count) in [("A", 3), ("B", 2), ("C", 1)] {
            pile.add(symbol(""), symbol(code), count);
        }
        // Always choosing the last place swaps a piece with itself: the order is kept.
        let kept = pile.shuffled(|n| n - 1).expect("shuffles");
        assert_eq!(kept, pile);
        // Always choosing the first place rotates the pile.
        let rotated = pile.shuffled(|_| 0).expect("shuffles");
        assert_ne!(rotated, pile);
        assert_eq!(rotated.total(), 6);
        for code in ["A", "B", "C"] {
            assert_eq!(rotated.count_of(code), pile.count_of(code));
        }
        let mut empty = Holder::default();
        assert_eq!(empty.shuffled(|_| 0), Some(Holder::default()));
        empty.add(symbol(""), symbol("A"), MAX_SHUFFLED + 1);
        assert!(empty.shuffled(|_| 0).is_none(), "too big to shuffle");
    }

    #[test]
    fn a_holder_saves_as_a_list() {
        let mut holder = Holder::default();
        holder.add(symbol("WHITE"), symbol("PAWN"), 2);
        let json = serde_json::to_value(&holder).expect("saves");
        assert_eq!(json[0]["owner"], "WHITE");
        assert_eq!(json[0]["count"], 2);
        let back: Holder = serde_json::from_value(json).expect("loads");
        assert_eq!(back, holder);
    }
}
