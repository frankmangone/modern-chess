//! The pieces of a position, borrowed.
//!
//! A way being worked out changes the position step by step, and its tests ask about the position
//! as the steps so far leave it. `PiecesView` answers that without copying the map of pieces: it
//! is the position the turn started from, with the way's changes laid over it (and, for a drop,
//! the piece held in the hand, which sits on the place `nowhere`).

use crate::logic::places::holders::{Held, Holder};
use crate::logic::play::ways::HolderChange;
use crate::logic::Piece;
use crate::shared::{BoardChange, IdMap, Position, Symbol};

/// A position: a map of pieces, optionally with changes laid over it. `Copy`: it only borrows.
#[derive(Clone, Copy)]
pub struct PiecesView<'a> {
    base: &'a IdMap<Position, Piece>,
    changes: &'a [BoardChange],
    held: Option<(&'a Position, &'a Piece)>,
    /// What the holders keep, with the changes of the way laid over it.
    holders: Option<&'a IdMap<Position, Holder>>,
    holder_changes: &'a [HolderChange],
}

impl<'a> PiecesView<'a> {
    /// The pieces of `base`, as they stand.
    pub fn of(base: &'a IdMap<Position, Piece>) -> Self {
        PiecesView {
            base,
            changes: &[],
            held: None,
            holders: None,
            holder_changes: &[],
        }
    }

    /// `base` with `changes` applied in order, later ones overwriting earlier ones.
    pub fn with(base: &'a IdMap<Position, Piece>, changes: &'a [BoardChange]) -> Self {
        PiecesView {
            base,
            changes,
            held: None,
            holders: None,
            holder_changes: &[],
        }
    }

    /// The same position with `piece` standing on `at` (the piece held in a hand), unless a
    /// change has taken that place.
    pub fn holding(mut self, at: &'a Position, piece: &'a Piece) -> Self {
        self.held = Some((at, piece));
        self
    }

    /// The same position with the holders `holders` keep, and `changes` made to them so far.
    pub fn with_holders(
        mut self,
        holders: &'a IdMap<Position, Holder>,
        changes: &'a [HolderChange],
    ) -> Self {
        self.holders = Some(holders);
        self.holder_changes = changes;
        self
    }

    /// What the holder at `at` keeps.
    fn kept(&self, at: &Position) -> Holder {
        let Some(holders) = self.holders else {
            return Holder::default();
        };
        let mut holder = holders.get(at).cloned().unwrap_or_default();
        for change in self.holder_changes.iter().filter(|change| change.at == *at) {
            holder.apply(change.owner, change.code, change.delta);
        }
        holder
    }

    /// How many pieces the holder at `at` keeps.
    pub fn kept_total(&self, at: &Position) -> u32 {
        if self.holders.is_none() {
            return 0;
        }
        self.kept(at).total()
    }

    /// How many pieces the holder at `at` keeps that pass `keep`.
    pub fn count_kept(&self, at: &Position, keep: impl Fn(&Held) -> bool) -> u32 {
        if self.holders.is_none() {
            return 0;
        }
        self.kept(at)
            .entries()
            .iter()
            .filter(|held| keep(held))
            .map(|held| held.count)
            .sum()
    }

    /// Everything the holders keep, as (place, owner, code, count), sorted by place, owner and
    /// code; places that keep nothing are left out.
    pub fn kept_entries(&self) -> Vec<(Position, Symbol, Symbol, u32)> {
        let Some(holders) = self.holders else {
            return Vec::new();
        };
        let mut places: Vec<Position> = holders.keys().copied().collect();
        for change in self.holder_changes {
            if !places.contains(&change.at) {
                places.push(change.at);
            }
        }
        let mut entries: Vec<(Position, Symbol, Symbol, u32)> = places
            .iter()
            .flat_map(|at| {
                self.kept(at)
                    .entries()
                    .iter()
                    .filter(|held| held.count > 0)
                    .map(|held| (*at, held.owner, held.code, held.count))
                    .collect::<Vec<_>>()
            })
            .collect();
        entries.sort_by(|a, b| {
            (a.0.as_str(), a.1.as_str(), a.2.as_str()).cmp(&(
                b.0.as_str(),
                b.1.as_str(),
                b.2.as_str(),
            ))
        });
        entries
    }

    fn touched(&self, at: &Position) -> bool {
        self.changes.iter().any(|change| change.position == *at)
    }

    /// The piece on `at`.
    pub fn get(&self, at: &Position) -> Option<&'a Piece> {
        if let Some(change) = self.changes.iter().rev().find(|c| c.position == *at) {
            return change.piece.as_ref();
        }
        if let Some((place, piece)) = self.held {
            if *place == *at {
                return Some(piece);
            }
        }
        self.base.get(at)
    }

    pub fn contains_key(&self, at: &Position) -> bool {
        self.get(at).is_some()
    }

    /// Every piece with its place, in no fixed order.
    pub fn iter(&self) -> impl Iterator<Item = (&'a Position, &'a Piece)> + 'a {
        let view = *self;
        let held = view.held.filter(|(place, _)| !view.touched(place));
        let from_base = view.base.iter().filter(move |(place, _)| {
            !view.touched(place) && !view.held.is_some_and(|(held, _)| held == *place)
        });
        let from_changes = view
            .changes
            .iter()
            .enumerate()
            .filter_map(move |(i, change)| {
                let piece = change.piece.as_ref()?;
                let replaced = view
                    .changes
                    .get(i + 1..)
                    .unwrap_or_default()
                    .iter()
                    .any(|later| later.position == change.position);
                (!replaced).then_some((&change.position, piece))
            });
        from_base.chain(from_changes).chain(held)
    }

    pub fn values(&self) -> impl Iterator<Item = &'a Piece> + 'a {
        self.iter().map(|(_, piece)| piece)
    }

    /// The position as an owned map, for the few uses that change it.
    pub fn to_map(&self) -> IdMap<Position, Piece> {
        let mut map = self.base.clone();
        if let Some((place, piece)) = self.held {
            map.insert(*place, piece.clone());
        }
        crate::logic::game::state::board_changes::apply_to_map(&mut map, self.changes);
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(entries: &[(&str, &str, &str)]) -> IdMap<Position, Piece> {
        entries
            .iter()
            .map(|(at, code, player)| (Position::new(at), Piece::new(*code, *player)))
            .collect()
    }

    fn codes(view: &PiecesView) -> Vec<String> {
        let mut found: Vec<String> = view
            .iter()
            .map(|(at, piece)| format!("{at}={}", piece.code))
            .collect();
        found.sort();
        found
    }

    #[test]
    fn a_plain_view_is_the_map() {
        let base = map(&[("0,0", "KING", "WHITE"), ("1,0", "ROOK", "BLACK")]);
        let view = PiecesView::of(&base);
        assert_eq!(view.get(&Position::new("1,0")).unwrap().code, "ROOK");
        assert!(view.get(&Position::new("2,0")).is_none());
        assert_eq!(codes(&view), ["0,0=KING", "1,0=ROOK"]);
    }

    #[test]
    fn changes_are_laid_over_the_map_in_order() {
        let base = map(&[("0,0", "KING", "WHITE"), ("1,0", "ROOK", "BLACK")]);
        let changes = vec![
            BoardChange::clear(&Position::new("0,0")),
            BoardChange::set_piece(Position::new("2,0"), Piece::new("KING", "WHITE")),
            BoardChange::set_piece(Position::new("1,0"), Piece::new("QUEEN", "WHITE")),
            BoardChange::set_piece(Position::new("2,0"), Piece::new("BISHOP", "WHITE")),
            BoardChange::clear(&Position::new("3,0")),
        ];
        let view = PiecesView::with(&base, &changes);
        assert!(view.get(&Position::new("0,0")).is_none());
        assert_eq!(view.get(&Position::new("1,0")).unwrap().code, "QUEEN");
        assert_eq!(view.get(&Position::new("2,0")).unwrap().code, "BISHOP");
        assert_eq!(codes(&view), ["1,0=QUEEN", "2,0=BISHOP"]);
        assert_eq!(view.to_map().len(), 2);
    }

    #[test]
    fn a_held_piece_replaces_a_base_entry_on_its_place() {
        let nowhere = Position::default();
        let mut base = map(&[("0,0", "KING", "WHITE")]);
        base.insert(nowhere, Piece::new("STALE", "WHITE"));
        let held = Piece::new("PAWN", "WHITE");
        let view = PiecesView::of(&base).holding(&nowhere, &held);
        assert_eq!(view.get(&nowhere).unwrap().code, "PAWN");
        assert_eq!(codes(&view), ["0,0=KING", "=PAWN"]);
        assert_eq!(view.to_map().get(&nowhere).unwrap().code, "PAWN");
    }

    #[test]
    fn a_held_piece_stands_on_its_place_unless_a_change_takes_it() {
        let base = map(&[("0,0", "KING", "WHITE")]);
        let nowhere = Position::default();
        let held = Piece::new("PAWN", "WHITE");
        let view = PiecesView::of(&base).holding(&nowhere, &held);
        assert_eq!(view.get(&nowhere).unwrap().code, "PAWN");
        assert_eq!(codes(&view), ["0,0=KING", "=PAWN"]);
        let changes = vec![BoardChange::clear(&nowhere)];
        let taken = PiecesView::with(&base, &changes).holding(&nowhere, &held);
        assert!(taken.get(&nowhere).is_none());
        assert_eq!(codes(&taken), ["0,0=KING"]);
    }
}
