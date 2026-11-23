//! Holders: places that keep counts of pieces instead of a piece. A bag (the hand among them) is
//! a holder off the board; a map place with a capacity above one is a holder on it.

use serde::{Deserialize, Serialize};

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
