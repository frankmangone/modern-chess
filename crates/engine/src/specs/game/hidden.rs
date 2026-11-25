//! Hidden information, read when the spec is read: what a bag may declare, and what the rules
//! may do with a bag the mover cannot see into.
//!
//! Two rules. A `HELD` choice may only name a bag the mover can see, always: a choice shows its
//! options outright. And on a bag marked `strict`, the rules may not read what the mover cannot
//! see: it is only the source or target of a `MOVE`, `REMOVE`, `PLACE`, `ROLL` or `SHUFFLE` with
//! no `what`, and no test, `COUNT`, `count` region, `FOLLOW`, `FOR_EACH` or `REPEATS` reads it, except
//! for its total when it shows its count. End checks read anything: they end the game, which shows
//! everything.

use super::action::{ActionSpec, ChoiceSpec, CountSpec, DoSpec, EffectSpec, IfSpec};
use super::bag::{BagFor, BagSpec, Visible};
use super::game::{GameSpec, GameSpecError};
use super::test_block::{AroundOf, PlaceSpec, RegionSpec, TestSpec};

/// Who moves where a step is read: the players of an action that names them, or whoever has
/// the turn (the other places that run steps and tests: `every_action`, `must`, a stage, a
/// piece's move).
#[derive(Clone, Copy)]
struct Mover<'a> {
    /// The players that may be the mover; empty for any player.
    only: &'a [String],
    /// Where the steps are, for a refusal to say.
    where_: &'a str,
}

impl BagSpec {
    /// Whether the mover can see what the bag `of` keeps. `MOVER`'s own bag, always. A named
    /// player's, only when the steps are for that player alone. The other player's, never.
    fn seen_by(&self, of: Option<&str>, mover: Mover<'_>) -> bool {
        match self.visible {
            Visible::All => true,
            Visible::Nobody => false,
            Visible::Owner => match of {
                Some("MOVER") => true,
                Some(name) => mover.only == [name.to_string()],
                None => false,
            },
        }
    }
}

impl GameSpec {
    /// The bags' own fields: what is written only where it means something.
    pub(super) fn validate_bag_visibility(&self) -> Result<(), GameSpecError> {
        for bag in &self.bags {
            let refused = |why: &str| {
                Err(GameSpecError::InvalidStep(format!(
                    "bag `{}`: {why}",
                    bag.name
                )))
            };
            if bag.visible.is_all() && bag.shows_count.is_some() {
                return refused(
                    "`shows_count` is written on a bag everyone sees into; it says whether the \
                     count shows when the contents do not, so write it with `visible` OWNER or NONE",
                );
            }
            if bag.visible.is_all() && bag.strict.is_some() {
                return refused(
                    "`strict` is written on a bag everyone sees into; there is nothing to keep \
                     the rules from reading, so write it with `visible` OWNER or NONE",
                );
            }
            if bag.visible == Visible::Owner && bag.owners != Some(BagFor::EachPlayer) {
                return refused(
                    "`visible` is OWNER on a shared bag, which belongs to nobody; write NONE to \
                     hide it from every player, or `\"for\": \"EACH_PLAYER\"`",
                );
            }
        }
        Ok(())
    }

    /// The rules about reading what a bag hides, over every place that runs steps and tests (an
    /// end check is left out: it ends the game).
    pub(super) fn validate_hidden_reads(&self) -> Result<(), GameSpecError> {
        if self.bags.iter().all(|bag| bag.visible.is_all()) {
            return Ok(());
        }
        for action in &self.actions {
            let where_ = format!("action {}", action.name);
            self.check_steps(&action.r#do, mover_of(action, &where_))?;
        }
        for (index, entry) in self.every_action.iter().enumerate() {
            let where_ = format!("entry {} of every_action", index + 1);
            self.check_steps(&entry.r#do, any_mover(&where_))?;
        }
        for (index, test) in self.must.iter().enumerate() {
            let where_ = format!("entry {} of must", index + 1);
            self.check_test(test, any_mover(&where_))?;
        }
        for stage in &self.turns.stages {
            if let Some(only) = &stage.only {
                let where_ = format!("the `only` of stage {}", stage.name);
                self.check_test(only, any_mover(&where_))?;
            }
        }
        for piece in &self.pieces {
            for movement in &piece.moves {
                let where_ = format!("move {} of piece {}", movement.id, piece.code);
                let mover = any_mover(&where_);
                self.check_steps(&movement.r#do, mover)?;
                if let Some(test) = movement.repeat.as_ref().and_then(|r| r.until.as_ref()) {
                    self.check_test(test, mover)?;
                }
            }
        }
        Ok(())
    }

    fn bag_named(&self, place: &PlaceSpec) -> Option<&BagSpec> {
        let name = place.bag.as_ref()?;
        self.bags.iter().find(|bag| bag.name == *name)
    }

    /// A refusal of reading `bag`, found in `mover.where_`.
    fn refuse_read(&self, bag: &BagSpec, doing: &str, mover: Mover<'_>) -> GameSpecError {
        GameSpecError::InvalidStep(format!(
            "{} {doing} the bag `{}`, which is strict and which the mover cannot see into; a \
             strict bag is only moved, removed, placed or shuffled without a `what`, and its \
             total is read only when `shows_count` is on (an end check may read it)",
            mover.where_, bag.name
        ))
    }

    /// Reads `place` as the rules would: a strict bag the mover cannot see is refused, unless
    /// only its total is read (`total_only`) and its count shows.
    fn check_read(
        &self,
        place: &PlaceSpec,
        total_only: bool,
        mover: Mover<'_>,
    ) -> Result<(), GameSpecError> {
        let Some(bag) = self.bag_named(place) else {
            return Ok(());
        };
        if !bag.is_strict() || bag.seen_by(place.of.as_deref(), mover) {
            return Ok(());
        }
        if total_only && bag.shows_count() {
            return Ok(());
        }
        Err(self.refuse_read(bag, "reads", mover))
    }

    fn check_steps(&self, steps: &[DoSpec], mover: Mover<'_>) -> Result<(), GameSpecError> {
        for step in steps {
            match step {
                DoSpec::Choose(choice) => self.check_choice(choice, mover)?,
                DoSpec::Test(test) => self.check_test(test, mover)?,
                DoSpec::Effect(effect) => self.check_effect(effect, mover)?,
                DoSpec::If(IfSpec { r#if, then, r#else }) => {
                    self.check_test(r#if, mover)?;
                    self.check_steps(then, mover)?;
                    self.check_steps(r#else, mover)?;
                }
            }
        }
        Ok(())
    }

    /// A `HELD` choice shows its options, so it may only name a bag the mover sees (not only
    /// a strict one). A `PLACE` choice reads the region it offers.
    fn check_choice(&self, choice: &ChoiceSpec, mover: Mover<'_>) -> Result<(), GameSpecError> {
        match choice {
            ChoiceSpec::Held { r#in, .. } => {
                let Some(bag) = self.bags.iter().find(|bag| bag.name == r#in.bag) else {
                    return Ok(());
                };
                if bag.seen_by(r#in.of.as_deref(), mover) {
                    return Ok(());
                }
                Err(GameSpecError::InvalidStep(format!(
                    "{} has a HELD choice in the bag `{}`, which the mover cannot see into; a \
                     choice shows its options, so it would show the contents. Choose from a bag \
                     the mover sees (`of` MOVER for a bag only its owner sees)",
                    mover.where_, bag.name
                )))
            }
            ChoiceSpec::Place { r#in, .. } => self.check_region(r#in, true, mover),
            ChoiceSpec::Piece { .. } | ChoiceSpec::Move { .. } | ChoiceSpec::Code { .. } => Ok(()),
        }
    }

    fn check_count(&self, count: &CountSpec, mover: Mover<'_>) -> Result<(), GameSpecError> {
        match count {
            CountSpec::Of(of) => self.check_region(&of.r#in, of.r#where.is_some(), mover),
            CountSpec::Number(_) | CountSpec::All => Ok(()),
        }
    }

    fn check_effect(&self, effect: &EffectSpec, mover: Mover<'_>) -> Result<(), GameSpecError> {
        match effect {
            EffectSpec::Move {
                from, what, count, ..
            } => {
                self.check_picking(from, what.is_some(), mover)?;
                if let Some(count) = count {
                    self.check_count(count, mover)?;
                }
                Ok(())
            }
            EffectSpec::Remove {
                at, what, count, ..
            } => {
                self.check_picking(at, what.is_some(), mover)?;
                if let Some(count) = count {
                    self.check_count(count, mover)?;
                }
                Ok(())
            }
            EffectSpec::Place { count, .. } => match count {
                Some(count) => self.check_count(count, mover),
                None => Ok(()),
            },
            EffectSpec::SetOwner { at }
            | EffectSpec::SetFlag { at, .. }
            | EffectSpec::SetCode { at, .. } => self.check_read(at, false, mover),
            EffectSpec::ForEach { r#in, r#do } => {
                self.check_region(r#in, true, mover)?;
                self.check_steps(r#do, mover)
            }
            EffectSpec::Roll { dice, .. } => match dice {
                Some(dice) => self.check_count(dice, mover),
                None => Ok(()),
            },
            EffectSpec::Add { .. }
            | EffectSpec::Reset { .. }
            | EffectSpec::Stage { .. }
            | EffectSpec::Ask { .. }
            | EffectSpec::Shuffle { .. } => Ok(()),
        }
    }

    /// Takes pieces out of `from`: only with a `what` is it picking among what the bag keeps.
    fn check_picking(
        &self,
        from: &PlaceSpec,
        has_what: bool,
        mover: Mover<'_>,
    ) -> Result<(), GameSpecError> {
        if has_what {
            self.check_read(from, false, mover)
        } else {
            Ok(())
        }
    }

    fn check_test(&self, test: &TestSpec, mover: Mover<'_>) -> Result<(), GameSpecError> {
        match test {
            TestSpec::All { of } | TestSpec::Any { of } => {
                of.iter().try_for_each(|test| self.check_test(test, mover))
            }
            TestSpec::Not { of } => self.check_test(of, mover),
            TestSpec::Count { r#in, r#where, .. } => {
                self.check_region(r#in, r#where.is_some(), mover)
            }
            TestSpec::Attacked { r#in, .. } => self.check_region(r#in, true, mover),
            TestSpec::InZone { at, .. } => self.check_read(at, false, mover),
            TestSpec::Same { a, b } => {
                self.check_read(a, false, mover)?;
                self.check_read(b, false, mover)
            }
            TestSpec::SomePlace { r#in, that } => {
                self.check_region(r#in, true, mover)?;
                self.check_test(that, mover)
            }
            // The position it compares is the whole of it, every bag's contents included.
            TestSpec::Repeats { .. } => match self.bags.iter().find(|bag| bag.is_strict()) {
                Some(bag) => Err(GameSpecError::InvalidStep(format!(
                    "{} has a REPEATS, which compares the whole position, the contents of the \
                     strict bag `{}` among them; leave it out, or let only an end check ask",
                    mover.where_, bag.name
                ))),
                None => Ok(()),
            },
            TestSpec::Removed { .. }
            | TestSpec::Moved { .. }
            | TestSpec::Stuck {}
            | TestSpec::Passes { .. }
            | TestSpec::Player { .. }
            | TestSpec::Counter { .. } => Ok(()),
        }
    }

    /// Reads a region. `by_kind` says the pieces are told apart (a filter), not only counted,
    /// which is more than a total.
    fn check_region(
        &self,
        region: &RegionSpec,
        by_kind: bool,
        mover: Mover<'_>,
    ) -> Result<(), GameSpecError> {
        match region {
            RegionSpec::At { at } => self.check_read(at, !by_kind, mover),
            RegionSpec::Path { from, to, .. } => {
                self.check_read(from, false, mover)?;
                self.check_read(to, false, mover)
            }
            RegionSpec::Around { of, .. } => match of {
                AroundOf::Place(place) => self.check_read(place, false, mover),
                AroundOf::Region(region) => self.check_region(region, true, mover),
            },
            RegionSpec::Group { of, .. } => self.check_read(of, false, mover),
            RegionSpec::Line { through, .. } => self.check_read(through, false, mover),
            RegionSpec::Follow { from, steps, .. } => {
                self.check_read(from, false, mover)?;
                self.check_count(steps, mover)
            }
            RegionSpec::Board { .. } => Ok(()),
        }
    }
}

fn mover_of<'a>(action: &'a ActionSpec, where_: &'a str) -> Mover<'a> {
    Mover {
        only: &action.players,
        where_,
    }
}

fn any_mover(where_: &str) -> Mover<'_> {
    Mover { only: &[], where_ }
}
