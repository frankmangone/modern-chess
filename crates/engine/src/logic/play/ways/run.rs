//! Running an action's steps into ways. A choice branches into one run per option, a test
//! ends the run when it fails, and an effect adds to the run's changes. A run that reaches
//! the end of its steps is a complete way.
//!
//! Tests are judged on a borrowed view of the position: the game's own pieces with the changes
//! made so far laid over them (see `PiecesView`), so no copy of the pieces is ever made.

use crate::logic::{PiecesView, Removal};
use crate::shared::IdMap;

use std::collections::HashSet;

use super::{Answer, ChoiceKind, CountChange, HolderChange, Rolled, Way};
use crate::shared::chance;
use crate::logic::places::holders::HolderOwner;
use crate::logic::places::holders::{Held, Holder};
use crate::logic::places::place::Place;
use crate::logic::rules::steps::{Action, AskStep, Choice, CodeRef, Count, EffectStep, Step};
use crate::logic::rules::test_block::{
    formed_places, judge, locate, matches, matches_held, region_places, Filter, Region, Test,
    TestContext,
};
use crate::logic::rules::test_block::{resolve_count, CountExpr};
use crate::logic::{Game, NodeId, Piece, PieceState};
use crate::shared::{BoardChange, Position, Symbol};

type Pieces = IdMap<Position, Piece>;

/// A way being worked out: what has been answered and changed so far.
#[derive(Clone, Default)]
struct Run {
    answers: Vec<Answer>,
    kinds: Vec<ChoiceKind>,
    changes: Vec<BoardChange>,
    start: Option<Position>,
    target: Option<Position>,
    /// Where the piece the `PIECE` choice picked (or the piece moved from the hand) stands
    /// now; `None` once it has left the board.
    picked: Option<Position>,
    /// The code of the piece a `HELD` choice took, until it leaves its bag, and the bag.
    hand: Option<String>,
    hand_bag: Option<Position>,
    /// Every bag a `HELD` choice has taken from so far, in order.
    held_bags: Vec<Position>,
    /// The code of the piece that left a bag by a drop.
    from_hand: Option<Symbol>,
    /// What the effects have added to and taken out of holders, in order.
    holder_changes: Vec<HolderChange>,
    /// The answers of the `CODE` choices, by name.
    chosen: Vec<(String, String)>,
    captures: bool,
    /// The pieces the changes have taken off the board, as they stood, in the order taken.
    removed: Vec<Removal>,
    /// The places the moves have carried a piece from, along the board.
    moved: Vec<NodeId>,
    /// The counter changes the effects have made.
    counts: Vec<CountChange>,
    /// Where the latest `MOVE` or `PLACE` put a piece, from the way before in a stage.
    last: Option<Position>,
    /// The stage the effects have put the turn in.
    stage: Option<Symbol>,
    /// The seats the effects have asked.
    ask: Option<AskStep>,
    /// The random numbers the effects have used so far.
    draws: u32,
    /// The dice rolled so far.
    rolled: Vec<Rolled>,
    /// The piles shuffled so far.
    shuffled: Vec<Position>,
    effects_before: Vec<(usize, usize)>,
    /// The piece a `HAND` choice took, standing on the place `nowhere` for the tests.
    held: Option<(Position, Piece)>,
}

impl Run {
    /// Gives `answer` to a choice of `kind`, noting how many changes came before it.
    fn answer(&mut self, answer: Answer, kind: ChoiceKind) {
        if !self.changes.is_empty() {
            self.effects_before
                .push((self.answers.len(), self.changes.len()));
        }
        self.answers.push(answer);
        self.kinds.push(kind);
    }

    fn push(&mut self, change: BoardChange) {
        self.changes.push(change);
    }

    /// The position as the changes so far leave it, borrowed from `base`.
    fn view<'a>(&'a self, base: &'a Pieces) -> PiecesView<'a> {
        let view = PiecesView::with(base, &self.changes);
        match &self.held {
            Some((place, piece)) => view.holding(place, piece),
            None => view,
        }
    }

    /// The piece on `at` as the changes so far leave it.
    fn piece_at<'a>(&'a self, base: &'a Pieces, at: &Position) -> Option<&'a Piece> {
        self.view(base).get(at)
    }

    /// The code `code` names: written, or the answer to the `CODE` choice of that name.
    fn code(&self, code: &CodeRef) -> Option<String> {
        match code {
            CodeRef::Code(code) => Some(code.clone()),
            CodeRef::Chosen(name) => self
                .chosen
                .iter()
                .rev()
                .find(|(chosen, _)| chosen == name)
                .map(|(_, code)| code.clone()),
        }
    }

    fn finish(self, action: &str) -> Way {
        Way {
            action: action.to_string(),
            answers: self.answers,
            kinds: self.kinds,
            changes: self.changes,
            from_hand: self.from_hand,
            holder_changes: self.holder_changes,
            captures: self.captures,
            removed: self.removed,
            moved: self.moved,
            counts: self.counts,
            start: self.start,
            last: self.last,
            stage: self.stage,
            ask: self.ask,
            held_bags: self.held_bags,
            draws: self.draws,
            rolled: self.rolled,
            shuffled: self.shuffled,
            effects_before: self.effects_before,
        }
    }
}

/// Runs the steps of one action for the player to move.
struct Runner<'g> {
    game: &'g Game,
    player: &'g str,
    action: &'g str,
    /// Where `START` points after a `HAND` choice: the piece in the mover's hand, on no place
    /// of the board (the empty name, as a drop's "from hand" in the history). Also what
    /// `TARGET` is read as before any choice sets it, which the spec's checker never lets a
    /// step read.
    nowhere: Position,
}

impl<'g> Runner<'g> {
    fn new(game: &'g Game, action: &'g str) -> Self {
        Runner {
            game,
            player: game.current_player_ref(),
            action,
            nowhere: Position::default(),
        }
    }

    /// Runs the steps left in `frames` (the innermost list last) on `run`, adding each complete
    /// way to `out`. `base` is the position before the turn's effects.
    fn run(&self, base: &Pieces, mut frames: Vec<&'g [Step]>, mut run: Run, out: &mut Vec<Way>) {
        while let Some(frame) = frames.last_mut() {
            let steps: &'g [Step] = *frame;
            let Some((step, rest)) = steps.split_first() else {
                frames.pop();
                continue;
            };
            *frame = rest;
            match step {
                Step::Test(test) => {
                    if !self.holds(test, base, &mut run, None) {
                        return;
                    }
                }
                Step::Effect(effect) => {
                    if !self.effect(effect, base, &mut run, None) {
                        return;
                    }
                }
                Step::If {
                    test,
                    then,
                    otherwise,
                } => {
                    let branch = if self.holds(test, base, &mut run, None) {
                        then
                    } else {
                        otherwise
                    };
                    frames.push(branch);
                }
                Step::Choose(choice) => return self.choose(choice, base, frames, run, out),
            }
        }
        out.push(run.finish(self.action));
    }

    /// Runs the steps of a `FOR_EACH` or of an `if` inside one, which ask nothing. Returns
    /// `false` when a test fails (the spec's checker allows none there).
    fn run_through(
        &self,
        steps: &'g [Step],
        base: &Pieces,
        run: &mut Run,
        each: Option<&Position>,
    ) -> bool {
        steps.iter().all(|step| match step {
            Step::Effect(effect) => self.effect(effect, base, run, each),
            Step::If {
                test,
                then,
                otherwise,
            } => {
                let branch = if self.holds(test, base, run, each) {
                    then
                } else {
                    otherwise
                };
                self.run_through(branch, base, run, each)
            }
            Step::Test(test) => self.holds(test, base, run, each),
            Step::Choose(_) => true,
        })
    }

    fn context<'c>(
        &'c self,
        base: &'c Pieces,
        run: &'c Run,
        each: Option<&'c Position>,
    ) -> TestContext<'c> {
        TestContext {
            game: self.game,
            pieces: run
                .view(base)
                .with_holders(&self.game.state.holders, &run.holder_changes),
            player: self.player,
            start: run.start.as_ref(),
            target: run.target.as_ref().unwrap_or(&self.nowhere),
            each,
            removed: &run.removed,
            moved: &run.moved,
            after_turn: None,
            last: run.last.as_ref(),
        }
    }

    /// Whether `test` holds on the position as the steps so far leave it.
    fn holds(&self, test: &Test, base: &Pieces, run: &mut Run, each: Option<&Position>) -> bool {
        judge(test, &self.context(base, run, each))
    }

    /// The places of `region`, on the position the steps so far have left when the region
    /// depends on the pieces, and on the position the action started from otherwise.
    fn region_places(
        &self,
        region: &Region,
        base: &Pieces,
        run: &mut Run,
        each: Option<&Position>,
    ) -> Vec<Position> {
        region_places(region, &self.context(base, run, each))
    }

    /// The place `place` names, if it can be formed.
    fn at(
        &self,
        place: &Place,
        base: &Pieces,
        run: &Run,
        each: Option<&Position>,
    ) -> Option<Position> {
        locate(place, &self.context(base, run, each)).cloned()
    }

    fn on_board(&self, position: &Position) -> bool {
        self.game.board.node(position).is_some()
    }

    /// The place `place` names, when it is a place of the board.
    fn board_place(
        &self,
        place: &Place,
        base: &Pieces,
        run: &Run,
        each: Option<&Position>,
    ) -> Option<Position> {
        self.at(place, base, run, each)
            .filter(|position| self.on_board(position))
    }

    /// Applies `effect`. Returns `false` when a test inside it fails.
    fn effect(
        &self,
        effect: &'g EffectStep,
        base: &Pieces,
        run: &mut Run,
        each: Option<&Position>,
    ) -> bool {
        match effect {
            EffectStep::Move {
                from,
                to,
                removed_to,
                what,
                count,
            } => {
                let from = self.at(from, base, run, each);
                let to = self.at(to, base, run, each);
                let removed_to = removed_to
                    .as_ref()
                    .and_then(|bag| self.at(bag, base, run, each));
                if let (Some(from), Some(to)) = (from, to) {
                    let Some(count) = self.resolved(count, base, run, each) else {
                        return false;
                    };
                    let before = (run.changes.len(), run.holder_changes.len());
                    let moved = self.move_pieces(
                        base,
                        run,
                        (&from, to),
                        removed_to.as_ref(),
                        what.as_ref(),
                        count,
                    );
                    if (run.changes.len(), run.holder_changes.len()) != before {
                        run.last = Some(to);
                    }
                    return moved;
                }
            }
            EffectStep::Remove {
                at,
                removed_to,
                what,
                count,
            } => {
                let at = self.at(at, base, run, each);
                let removed_to = removed_to
                    .as_ref()
                    .and_then(|bag| self.at(bag, base, run, each));
                if let Some(at) = at {
                    let Some(count) = self.resolved(count, base, run, each) else {
                        return false;
                    };
                    return if self.is_holder(&at) {
                        self.remove_from_holder(run, &at, removed_to.as_ref(), what.as_ref(), count)
                    } else if self.on_board(&at) {
                        self.remove(base, run, &at, removed_to.as_ref())
                    } else {
                        true
                    };
                }
            }
            EffectStep::Put { at, code, count } => {
                let at = self.at(at, base, run, each);
                if let (Some(at), Some(code)) = (at, run.code(code)) {
                    run.last = Some(at);
                    if self.is_holder(&at) {
                        let Some(Count::Number(number)) = self.resolved(count, base, run, each)
                        else {
                            return false;
                        };
                        return self.put_in(
                            run,
                            &at,
                            Symbol::new(self.player),
                            Symbol::new(&code),
                            number,
                        );
                    }
                    if self.on_board(&at) {
                        self.remove(base, run, &at, None);
                        run.push(BoardChange::set_piece(at, self.new_piece(code)));
                    }
                }
            }
            EffectStep::SetOwner(at) => {
                let at = self.board_place(at, base, run, each);
                let piece = at.as_ref().and_then(|at| run.piece_at(base, at));
                if let (Some(at), Some(piece)) = (&at, piece) {
                    if piece.player != self.player {
                        let mut owned = piece.clone();
                        owned.player = Symbol::new(self.player);
                        run.push(BoardChange::set_piece(*at, owned));
                    }
                }
            }
            EffectStep::SetCode { at, code } => {
                let at = self.board_place(at, base, run, each);
                let owner = at
                    .as_ref()
                    .and_then(|at| run.piece_at(base, at))
                    .map(|piece| piece.player);
                if let (Some(at), Some(owner), Some(code)) = (at, owner, run.code(code)) {
                    run.push(BoardChange::set_piece(at, Piece::new(code, owner)));
                }
            }
            EffectStep::SetFlag { at, flag, turns } => {
                let at = self.board_place(at, base, run, each);
                let piece = at.as_ref().and_then(|at| run.piece_at(base, at)).cloned();
                if let (Some(at), Some(mut piece)) = (at, piece) {
                    let value = turns.map_or(PieceState::Blank, |turns| {
                        PieceState::Uint(u16::from(turns))
                    });
                    piece.state.insert(*flag, value);
                    run.push(BoardChange::set_piece(at, piece));
                }
            }
            EffectStep::ForEach { region, steps } => {
                let places = if matches!(region, Region::Follow { .. }) {
                    // A walk that meets a dead end, or is too long, makes no way at all,
                    // instead of a turn that does nothing.
                    match formed_places(region, &self.context(base, run, each)) {
                        Some(places) => places,
                        None => return false,
                    }
                } else {
                    self.region_places(region, base, run, each)
                };
                for place in &places {
                    if !self.run_through(steps, base, run, Some(place)) {
                        return false;
                    }
                }
            }
            EffectStep::Add { counter, by, all } => run.counts.push(CountChange {
                counter: counter.clone(),
                all: *all,
                reset: false,
                by: *by,
            }),
            EffectStep::Reset { counter, all } => run.counts.push(CountChange {
                counter: counter.clone(),
                all: *all,
                reset: true,
                by: 0,
            }),
            EffectStep::Stage(name) => {
                // A way hands the turn on one way: the same player again (`STAGE`), or other
                // players first (`ASK`). The checker refuses a way that can run both.
                if run.ask.is_some() {
                    return false;
                }
                run.stage = Some(*name);
            }
            EffectStep::Ask(ask) => {
                if run.stage.is_some() {
                    return false;
                }
                run.ask = Some(ask.clone());
            }
            EffectStep::Roll { at, dice, faces } => {
                let Some(at) = self.at(at, base, run, each) else {
                    return true;
                };
                let Some(Count::Number(dice)) = self.resolved(dice, base, run, each) else {
                    return false;
                };
                if dice > chance::MAX_DICE || faces.is_empty() {
                    return false;
                }
                let sides = u32::try_from(faces.len()).unwrap_or(u32::MAX);
                for _ in 0..dice {
                    let landed = chance::below(self.draw(run), sides) as usize;
                    let Some(face) = faces.get(landed) else {
                        return false;
                    };
                    for (code, count) in face {
                        if !self.put_in(run, &at, Symbol::new(self.player), *code, *count) {
                            return false;
                        }
                    }
                    run.rolled.push(Rolled {
                        at,
                        face: face.clone(),
                    });
                }
            }
            EffectStep::Shuffle(at) => {
                let Some(at) = self.at(at, base, run, each) else {
                    return true;
                };
                let mut used = 0u32;
                let (seed, first) = self.chance_at(run);
                let shuffled = self.holder_now(run, &at).shuffled(|bound| {
                    let x = chance::draw(seed, first.saturating_add(u64::from(used)));
                    used = used.saturating_add(1);
                    chance::below(x, bound)
                });
                let Some(shuffled) = shuffled else {
                    return false;
                };
                run.draws = run.draws.saturating_add(used);
                run.holder_changes
                    .push(HolderChange::replacing(at, shuffled));
                run.shuffled.push(at);
            }
        }
        true
    }

    /// The seed of the match and the index of the next random number the way would use.
    fn chance_at(&self, run: &Run) -> (u64, u64) {
        (
            self.game.state.seed.unwrap_or(0),
            self.game.state.draws.saturating_add(u64::from(run.draws)),
        )
    }

    /// The next random number of the way, which has now used it.
    fn draw(&self, run: &mut Run) -> u64 {
        let (seed, index) = self.chance_at(run);
        run.draws = run.draws.saturating_add(1);
        chance::draw(seed, index)
    }

    fn new_piece(&self, code: String) -> Piece {
        Piece::new(code, self.player.to_string())
    }

    /// Takes the piece on `at`, if any, off the board, noting it among the removed. With
    /// `removed_to`, a piece of another player goes to that bag, as its `captured_as`. Returns
    /// `false` when the bag has no room.
    fn remove(
        &self,
        base: &Pieces,
        run: &mut Run,
        at: &Position,
        removed_to: Option<&Position>,
    ) -> bool {
        let Some(piece) = run.piece_at(base, at) else {
            return true;
        };
        let other_player = piece.player != self.player;
        let removal = Removal::from(piece);
        let sent = removed_to.filter(|_| other_player).map(|bag| {
            let code = piece.code;
            let kept_as = self.game.rules.captured_as.get(&code).copied().flatten();
            (*bag, piece.player, kept_as.unwrap_or(code))
        });
        run.captures |= other_player;
        run.removed.push(removal);
        if run.picked.as_ref() == Some(at) {
            run.picked = None;
        }
        run.push(BoardChange::clear(at));
        match sent {
            Some((bag, owner, code)) => self.put_in(run, &bag, owner, code, 1),
            None => true,
        }
    }

    /// A count as written, `ALL` or the number counted in the position as the steps so far
    /// leave it; `None` when what it counts cannot be formed.
    fn resolved(
        &self,
        count: &CountExpr,
        base: &Pieces,
        run: &Run,
        each: Option<&Position>,
    ) -> Option<Count> {
        match count {
            CountExpr::All => Some(Count::All),
            other => resolve_count(other, &self.context(base, run, each)).map(Count::Number),
        }
    }

    fn is_holder(&self, at: &Position) -> bool {
        self.game.rules.holders.contains_key(at)
    }

    /// What the holder `at` keeps as the steps so far leave it.
    fn holder_now(&self, run: &Run, at: &Position) -> Holder {
        let mut holder = self.game.state.holders.get(at).cloned().unwrap_or_default();
        for change in run.holder_changes.iter().filter(|change| change.at == *at) {
            change.apply_to(&mut holder);
        }
        holder
    }

    /// Puts `count` pieces of `code` owned by `owner` in the holder `at`: they become the
    /// player's in a player's bag and nobody's in a shared one. Returns `false`, putting
    /// nothing, when they would pass its capacity.
    fn put_in(
        &self,
        run: &mut Run,
        at: &Position,
        owner: Symbol,
        code: Symbol,
        count: u32,
    ) -> bool {
        let Some(info) = self.game.rules.holders.get(at) else {
            return false;
        };
        if info.capacity.is_some_and(|capacity| {
            self.holder_now(run, at).total().saturating_add(count) > capacity
        }) {
            return false;
        }
        let Ok(delta) = i32::try_from(count) else {
            return false;
        };
        let owner = match info.owner {
            HolderOwner::Player(player) => player,
            HolderOwner::Shared => Symbol::new(""),
            HolderOwner::Place => owner,
        };
        if info.ordered {
            let mut pile = self.holder_now(run, at);
            pile.push_top(owner, code, count);
            run.holder_changes.push(HolderChange::replacing(*at, pile));
            return true;
        }
        run.holder_changes.push(HolderChange {
            at: *at,
            owner,
            code,
            delta,
            contents: None,
        });
        true
    }

    /// Takes `count` pieces that pass `what` out of the holder `at`, from the first owner that
    /// arrived and on. `None`, taking nothing, when fewer are there.
    fn take_out(
        &self,
        run: &mut Run,
        at: &Position,
        what: Option<&Filter>,
        count: Count,
    ) -> Option<Vec<Held>> {
        if self.game.rules.holders.get(at).is_some_and(|info| info.ordered) {
            return self.take_from_pile(run, at, count);
        }
        let holder = self.holder_now(run, at);
        let wanted: Vec<&Held> = holder
            .entries()
            .iter()
            .filter(|held| what.is_none_or(|filter| matches_held(filter, held, self.player)))
            .collect();
        let available: u32 = wanted.iter().map(|held| held.count).sum();
        let number = match count {
            Count::All => available,
            Count::Number(number) => number,
        };
        if available < number {
            return None;
        }
        let mut left = number;
        let mut taken = Vec::new();
        for held in wanted {
            let part = held.count.min(left);
            if part == 0 {
                continue;
            }
            left -= part;
            run.holder_changes.push(HolderChange {
                at: *at,
                owner: held.owner,
                code: held.code,
                delta: -i32::try_from(part).ok()?,
                contents: None,
            });
            taken.push(Held {
                owner: held.owner,
                code: held.code,
                count: part,
            });
        }
        Some(taken)
    }

    /// Takes the `count` pieces on top of the pile `at` (a spec never filters them: a `what` on a
    /// pile is refused when it is read). `None`, taking nothing, when fewer are there.
    fn take_from_pile(&self, run: &mut Run, at: &Position, count: Count) -> Option<Vec<Held>> {
        let mut pile = self.holder_now(run, at);
        let number = match count {
            Count::All => pile.total(),
            Count::Number(number) => number,
        };
        let taken = pile.pop_top(number)?;
        run.holder_changes.push(HolderChange::replacing(*at, pile));
        Some(taken)
    }

    /// A `REMOVE` of pieces of a holder: they leave it, for `removed_to` if there is one. Returns
    /// `false` when fewer are there than the count, or the bag has no room.
    fn remove_from_holder(
        &self,
        run: &mut Run,
        at: &Position,
        removed_to: Option<&Position>,
        what: Option<&Filter>,
        count: Count,
    ) -> bool {
        let Some(taken) = self.take_out(run, at, what, count) else {
            return false;
        };
        let Some(bag) = removed_to else {
            return true;
        };
        taken.iter().all(|held| {
            let kept_as = self
                .game
                .rules
                .captured_as
                .get(&held.code)
                .copied()
                .flatten();
            self.put_in(
                run,
                bag,
                held.owner,
                kept_as.unwrap_or(held.code),
                held.count,
            )
        })
    }

    /// The pieces on `from` go to `to`, removing what stood there (for `removed_to` if the
    /// piece removed goes anywhere). A place that holds many takes counts and gives them: a
    /// piece moved onto it becomes a count, and one moved off it is a new, unmoved piece. From a
    /// held piece (a `HELD` choice), one of its code leaves its bag. Only the picked piece
    /// counts a move. Returns `false` when the move cannot be made: too few pieces in a holder,
    /// or no room in one.
    fn move_pieces(
        &self,
        base: &Pieces,
        run: &mut Run,
        (from, to): (&Position, Position),
        removed_to: Option<&Position>,
        what: Option<&Filter>,
        count: Count,
    ) -> bool {
        if let Some(code) = run.hand.clone().filter(|_| *from == self.nowhere) {
            let (Some(bag), code_symbol) = (run.hand_bag, Symbol::new(&code)) else {
                return false;
            };
            let only_that = Filter {
                codes: vec![code_symbol],
                ..Filter::default()
            };
            let Some(taken) = self.take_out(run, &bag, Some(&only_that), Count::Number(1)) else {
                return false;
            };
            let owner = taken
                .first()
                .map(|held| held.owner)
                .filter(|owner| !owner.as_str().is_empty())
                .unwrap_or_else(|| Symbol::new(self.player));
            run.hand = None;
            run.from_hand = Some(code_symbol);
            if self.is_holder(&to) {
                return self.put_in(run, &to, owner, code_symbol, 1);
            }
            let removed = self.remove(base, run, &to, removed_to);
            run.push(BoardChange::set_piece(to, Piece::new(code_symbol, owner)));
            run.picked = Some(to);
            return removed;
        }
        match (self.is_holder(from), self.is_holder(&to)) {
            (false, false) => {
                if *from == to || !self.on_board(from) || !self.on_board(&to) {
                    return true;
                }
                let Some(mut piece) = run.piece_at(base, from).cloned() else {
                    return true;
                };
                if let Some(node) = self.game.board.node(from) {
                    run.moved.push(node);
                }
                let removed = self.remove(base, run, &to, removed_to);
                run.push(BoardChange::clear(from));
                if run.picked.as_ref() == Some(from) {
                    piece.total_moves = piece.total_moves.saturating_add(1);
                    run.picked = Some(to);
                }
                run.push(BoardChange::set_piece(to, piece));
                removed
            }
            (false, true) => {
                if !self.on_board(from) {
                    return true;
                }
                let Some(piece) = run.piece_at(base, from).cloned() else {
                    return true;
                };
                if !self.put_in(run, &to, piece.player, piece.code, 1) {
                    return false;
                }
                if run.picked.as_ref() == Some(from) {
                    run.picked = None;
                }
                run.push(BoardChange::clear(from));
                true
            }
            (true, false) => {
                if !self.on_board(&to) || count != Count::Number(1) {
                    return false;
                }
                let Some(taken) = self.take_out(run, from, what, count) else {
                    return false;
                };
                let Some(held) = taken.first() else {
                    return false;
                };
                let owner = if held.owner.as_str().is_empty() {
                    Symbol::new(self.player)
                } else {
                    held.owner
                };
                let removed = self.remove(base, run, &to, removed_to);
                run.push(BoardChange::set_piece(to, Piece::new(held.code, owner)));
                removed
            }
            (true, true) => {
                let Some(mut taken) = self.take_out(run, from, what, count) else {
                    return false;
                };
                // Put on top one run after another, a pile would turn the order over.
                if self.game.rules.holders.get(&to).is_some_and(|info| info.ordered) {
                    taken.reverse();
                }
                taken
                    .iter()
                    .all(|held| self.put_in(run, &to, held.owner, held.code, held.count))
            }
        }
    }

    /// Runs one branch for each option of `choice`.
    fn choose(
        &self,
        choice: &'g Choice,
        base: &Pieces,
        frames: Vec<&'g [Step]>,
        mut run: Run,
        out: &mut Vec<Way>,
    ) {
        match choice {
            Choice::Piece(filter) => {
                let places = self.pieces_matching(filter, base, &mut run);
                self.fork(base, frames, run, places, out, |run, _, place| {
                    run.answer(Answer::Place(place), ChoiceKind::Piece);
                    run.start = Some(place);
                    run.picked = Some(place);
                    run.hand = None;
                });
            }
            Choice::Move => {
                let moves = self.moves_of_start(base, &mut run);
                self.fork(
                    base,
                    frames,
                    run,
                    moves,
                    out,
                    |run, frames, (steps, target)| {
                        run.answer(Answer::Place(target), ChoiceKind::Move);
                        run.target = Some(target);
                        frames.push(steps);
                    },
                );
            }
            Choice::Place(region) => {
                let places = self.region_places(region, base, &mut run, None);
                self.fork(base, frames, run, places, out, |run, _, place| {
                    run.answer(Answer::Place(place), ChoiceKind::Place);
                    run.target = Some(place);
                });
            }
            Choice::Code { name, options } => {
                self.fork(base, frames, run, options.clone(), out, |run, _, code| {
                    run.answer(Answer::Code(code.clone()), ChoiceKind::Code);
                    run.chosen.push((name.clone(), code));
                });
            }
            Choice::Held { bag, codes } => {
                let bag = self.at(bag, base, &run, None);
                self.choose_hand(bag, codes, base, frames, run, out);
            }
        }
    }

    /// Runs one branch per option, each from a copy of `run` and `frames` (the last option
    /// takes them), after `answer` has given it its option.
    fn fork<T>(
        &self,
        base: &Pieces,
        mut frames: Vec<&'g [Step]>,
        mut run: Run,
        options: Vec<T>,
        out: &mut Vec<Way>,
        mut answer: impl FnMut(&mut Run, &mut Vec<&'g [Step]>, T),
    ) {
        let last = options.len().saturating_sub(1);
        for (index, option) in options.into_iter().enumerate() {
            let (mut next, mut next_frames) = if index == last {
                (std::mem::take(&mut run), std::mem::take(&mut frames))
            } else {
                (run.clone(), frames.clone())
            };
            answer(&mut next, &mut next_frames, option);
            self.run(base, next_frames, next, out);
        }
    }

    /// A `HAND` choice: one branch per code in the mover's hand. `START` is the piece in the
    /// hand, which the tests see on the place `nowhere`.
    fn choose_hand(
        &self,
        bag: Option<Position>,
        codes: &[String],
        base: &Pieces,
        frames: Vec<&'g [Step]>,
        run: Run,
        out: &mut Vec<Way>,
    ) {
        let Some(bag) = bag else {
            return;
        };
        let mut held: Vec<String> = Some(self.holder_now(&run, &bag))
            .into_iter()
            .flat_map(|held| held.codes())
            .filter(|(code, count)| {
                *count > 0 && (codes.is_empty() || codes.iter().any(|wanted| code == wanted))
            })
            .map(|(code, _)| code.to_string())
            .collect();
        held.sort();
        for code in held {
            let code = &code;
            let in_hand = self.new_piece(code.clone());
            let mut next = run.clone();
            next.held = Some((self.nowhere, in_hand));
            next.answer(Answer::Held(code.clone()), ChoiceKind::Held);
            next.start = Some(self.nowhere);
            next.picked = None;
            next.hand = Some(code.clone());
            next.hand_bag = Some(bag);
            next.held_bags.push(bag);
            self.run(base, frames.clone(), next, out);
        }
    }

    /// The places of the board holding a piece that passes `filter`, in board order.
    fn pieces_matching(&self, filter: &Filter, base: &Pieces, run: &mut Run) -> Vec<Position> {
        let pieces = run.view(base);
        let board = &self.game.board;
        let mut places: Vec<_> = pieces
            .iter()
            .filter(|(_, piece)| matches(filter, piece, self.player))
            .filter_map(|(place, _)| board.node(place).map(|node| (node, *place)))
            .collect();
        places.sort_unstable_by_key(|(node, _)| *node);
        places.into_iter().map(|(_, place)| place).collect()
    }

    /// Each move of the piece on `START` with each place it reaches, in spec order.
    fn moves_of_start(&self, base: &Pieces, run: &mut Run) -> Vec<(&'g [Step], Position)> {
        let pieces = run
            .view(base)
            .with_holders(&self.game.state.holders, &run.holder_changes);
        let Some(start) = &run.start else {
            return Vec::new();
        };
        let blueprint = pieces
            .get(start)
            .and_then(|piece| self.game.rules.blueprints.get(&piece.code));
        let Some(blueprint) = blueprint else {
            return Vec::new();
        };
        let mut moves = Vec::new();
        for movement in &blueprint.move_blueprints {
            let Some(steps) = movement.steps.get(self.player) else {
                continue;
            };
            let targets = movement.candidate_targets(self.player, start, pieces, self.game);
            moves.extend(targets.into_iter().map(|target| (steps.as_slice(), target)));
        }
        moves
    }
}

/// Keeps, among the ways from `first` on (one action's), the first of those with equal
/// answers.
fn keep_first_of_equal_answers(ways: &mut Vec<Way>, first: usize) {
    let mut seen: HashSet<Vec<Answer>> = HashSet::new();
    let mut index = 0;
    ways.retain(|way| {
        let keep = index < first || seen.insert(way.answers.clone());
        index += 1;
        keep
    });
}

impl Game {
    /// The ways of every action the player to move may take, in spec order, before the rules
    /// that apply to every move.
    pub(super) fn run_actions(&self) -> Vec<Way> {
        let player = self.current_player_ref();
        let mut ways = Vec::new();
        for action in &self.rules.actions {
            if !self.offers(action) {
                continue;
            }
            let Some(steps) = action.steps.get(player) else {
                continue;
            };
            let first = ways.len();
            let runner = Runner::new(self, &action.name);
            runner.run(
                &self.state.pieces,
                vec![steps.as_slice()],
                Run {
                    last: self.state.stage.as_ref().and_then(|stage| stage.last),
                    ..Run::default()
                },
                &mut ways,
            );
            keep_first_of_equal_answers(&mut ways, first);
        }
        ways
    }

    /// Whether `action` is offered where the turn is: an action that belongs to a stage only in
    /// that stage, and any other in an ordinary turn and in the stages no action belongs to.
    fn offers(&self, action: &Action) -> bool {
        let in_stage = self.state.stage.as_ref().map(|stage| stage.name);
        match (action.stage, in_stage) {
            (Some(own), Some(now)) => own == now,
            (Some(_), None) => false,
            (None, Some(now)) => !self
                .rules
                .stages
                .iter()
                .any(|rule| rule.name == now && rule.has_actions),
            (None, None) => true,
        }
    }

    /// The ways of moving the piece on `from` to `target` by its moves, in spec order, before
    /// the rules that apply to every move. Their action has no name.
    pub(crate) fn piece_move_ways(&self, from: &Position, target: &Position) -> Vec<Way> {
        let runner = Runner::new(self, "");
        let player = runner.player;
        let blueprint = self
            .state
            .pieces
            .get(from)
            .and_then(|piece| self.rules.blueprints.get(&piece.code));
        let Some(blueprint) = blueprint else {
            return Vec::new();
        };
        let mut ways = Vec::new();
        for movement in &blueprint.move_blueprints {
            let Some(steps) = movement.steps.get(player) else {
                continue;
            };
            let pieces = PiecesView::of(&self.state.pieces).with_holders(&self.state.holders, &[]);
            let targets = movement.candidate_targets(player, from, pieces, self);
            if !targets.contains(target) {
                continue;
            }
            let run = Run {
                answers: vec![Answer::Place(*from), Answer::Place(*target)],
                kinds: vec![ChoiceKind::Piece, ChoiceKind::Move],
                start: Some(*from),
                target: Some(*target),
                picked: Some(*from),
                ..Run::default()
            };
            runner.run(&self.state.pieces, vec![steps.as_slice()], run, &mut ways);
        }
        ways
    }
}
