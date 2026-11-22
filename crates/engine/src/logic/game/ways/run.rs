//! Running an action's steps into ways. A choice branches into one run per option, a test
//! ends the run when it fails, and an effect adds to the run's changes. A run that reaches
//! the end of its steps is a complete way.
//!
//! Tests before the first effect are judged on the game's own pieces, without copying them.
//! After an effect, a test that looks at pieces is judged on the position the changes have
//! made, built once for the run and kept up to date from then on. Effects read single places
//! from the changes so far, so they never need it.

use std::collections::{HashMap, HashSet};

use super::{Answer, ChoiceKind, Way};
use crate::logic::blueprint::steps::{Choice, CodeRef, EffectStep, Step};
use crate::logic::blueprint::test_block::{
    judge, locate, matches, region_places, Filter, Place, Test, TestContext,
};
use crate::logic::game::board_changes;
use crate::logic::{Game, Piece, PieceState};
use crate::shared::{BoardChange, Position};

type Pieces = HashMap<Position, Piece>;

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
    /// The code of the piece a `HAND` choice took, until it leaves the hand.
    hand: Option<String>,
    from_hand: Option<String>,
    /// The answers of the `CODE` choices, by name.
    chosen: Vec<(String, String)>,
    captures: bool,
    ends_turn: bool,
    effects_before: Vec<(usize, usize)>,
    /// The position the changes have made, once a test needed it; every later change is
    /// applied to it as well.
    after: Option<Pieces>,
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
        if let Some(after) = &mut self.after {
            board_changes::apply_to_map(after, std::slice::from_ref(&change));
        }
        self.changes.push(change);
    }

    /// The piece on `at` as the changes so far leave it.
    fn piece_at<'a>(&'a self, base: &'a Pieces, at: &Position) -> Option<&'a Piece> {
        if let Some(after) = &self.after {
            return after.get(at);
        }
        match self
            .changes
            .iter()
            .rev()
            .find(|change| change.position == *at)
        {
            Some(change) => change.piece.as_ref(),
            None => base.get(at),
        }
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
            captures: self.captures,
            ends_turn: self.ends_turn,
            lands: self.picked,
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
        pieces: &'c Pieces,
        run: &'c Run,
        each: Option<&'c Position>,
    ) -> TestContext<'c> {
        TestContext {
            game: self.game,
            pieces,
            player: self.player,
            start: run.start.as_ref(),
            target: run.target.as_ref().unwrap_or(&self.nowhere),
            each,
        }
    }

    /// Whether `test` holds on the position as the steps so far leave it.
    fn holds(&self, test: &Test, base: &Pieces, run: &mut Run, each: Option<&Position>) -> bool {
        if reads_pieces(test) {
            self.materialise(base, run);
        }
        let pieces = run.after.as_ref().unwrap_or(base);
        judge(test, &self.context(pieces, run, each))
    }

    /// Builds the position the changes have made, once, when there are changes.
    fn materialise(&self, base: &Pieces, run: &mut Run) {
        if run.after.is_none() && !run.changes.is_empty() {
            run.after = Some(board_changes::simulate_from(base, &run.changes));
        }
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
            EffectStep::Move { from, to } => {
                let from = self.at(from, base, run, each);
                let to = self.board_place(to, base, run, each);
                if let (Some(from), Some(to)) = (from, to) {
                    self.move_piece(base, run, &from, to);
                }
            }
            EffectStep::Remove(at) => {
                if let Some(at) = self.board_place(at, base, run, each) {
                    self.remove(base, run, &at);
                }
            }
            EffectStep::Put { at, code } => {
                let at = self.board_place(at, base, run, each);
                if let (Some(at), Some(code)) = (at, run.code(code)) {
                    self.remove(base, run, &at);
                    run.push(BoardChange::set_piece(at, self.new_piece(code)));
                }
            }
            EffectStep::SetOwner(at) => {
                let at = self.board_place(at, base, run, each);
                let piece = at.as_ref().and_then(|at| run.piece_at(base, at));
                if let (Some(at), Some(piece)) = (&at, piece) {
                    if piece.player != self.player {
                        let mut owned = piece.clone();
                        owned.player = self.player.to_string();
                        run.push(BoardChange::set_piece(at.clone(), owned));
                    }
                }
            }
            EffectStep::SetCode { at, code } => {
                let at = self.board_place(at, base, run, each);
                let owner = at
                    .as_ref()
                    .and_then(|at| run.piece_at(base, at))
                    .map(|piece| piece.player.clone());
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
                    piece.state.insert(flag.clone(), value);
                    run.push(BoardChange::set_piece(at, piece));
                }
            }
            EffectStep::ForEach { region, steps } => {
                let places = region_places(region, &self.context(base, run, each));
                for place in &places {
                    if !self.run_through(steps, base, run, Some(place)) {
                        return false;
                    }
                }
            }
            EffectStep::EndTurn => run.ends_turn = true,
        }
        true
    }

    fn new_piece(&self, code: String) -> Piece {
        Piece::new(code, self.player.to_string())
    }

    /// Takes the piece on `at`, if any, off the board.
    fn remove(&self, base: &Pieces, run: &mut Run, at: &Position) {
        let Some(owner) = run
            .piece_at(base, at)
            .map(|piece| piece.player != self.player)
        else {
            return;
        };
        run.captures |= owner;
        if run.picked.as_ref() == Some(at) {
            run.picked = None;
        }
        run.push(BoardChange::clear(at));
    }

    /// The piece on `from` goes to `to`, removing what stood there. From the hand, a new piece
    /// of the chosen code enters `to`. Only the picked piece counts a move.
    fn move_piece(&self, base: &Pieces, run: &mut Run, from: &Position, to: Position) {
        if let Some(code) = run.hand.clone().filter(|_| *from == self.nowhere) {
            self.remove(base, run, &to);
            run.push(BoardChange::set_piece(
                to.clone(),
                self.new_piece(code.clone()),
            ));
            run.hand = None;
            run.from_hand = Some(code);
            run.picked = Some(to);
            return;
        }
        if *from == to || !self.on_board(from) {
            return;
        }
        let Some(mut piece) = run.piece_at(base, from).cloned() else {
            return;
        };
        self.remove(base, run, &to);
        run.push(BoardChange::clear(from));
        if run.picked.as_ref() == Some(from) {
            piece.total_moves = piece.total_moves.saturating_add(1);
            run.picked = Some(to.clone());
        }
        run.push(BoardChange::set_piece(to, piece));
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
                    run.answer(Answer::Place(place.clone()), ChoiceKind::Piece);
                    run.start = Some(place.clone());
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
                        run.answer(Answer::Place(target.clone()), ChoiceKind::Move);
                        run.target = Some(target);
                        frames.push(steps);
                    },
                );
            }
            Choice::Place(region) => {
                let places = region_places(region, &self.context(base, &run, None));
                self.fork(base, frames, run, places, out, |run, _, place| {
                    run.answer(Answer::Place(place.clone()), ChoiceKind::Place);
                    run.target = Some(place);
                });
            }
            Choice::Code { name, options } => {
                self.fork(base, frames, run, options.clone(), out, |run, _, code| {
                    run.answer(Answer::Code(code.clone()), ChoiceKind::Code);
                    run.chosen.push((name.clone(), code));
                });
            }
            Choice::Hand(codes) => self.choose_hand(codes, base, frames, run, out),
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
        codes: &[String],
        base: &Pieces,
        frames: Vec<&'g [Step]>,
        run: Run,
        out: &mut Vec<Way>,
    ) {
        let mut held: Vec<&String> = self
            .game
            .state
            .hand
            .get(self.player)
            .into_iter()
            .flatten()
            .filter(|(code, count)| **count > 0 && (codes.is_empty() || codes.contains(code)))
            .map(|(code, _)| code)
            .collect();
        held.sort();
        for code in held {
            let in_hand = self.new_piece(code.clone());
            let mut with_hand = base.clone();
            with_hand.insert(self.nowhere.clone(), in_hand.clone());
            let mut next = run.clone();
            if let Some(after) = &mut next.after {
                after.insert(self.nowhere.clone(), in_hand);
            }
            next.answer(Answer::Hand(code.clone()), ChoiceKind::Hand);
            next.start = Some(self.nowhere.clone());
            next.picked = None;
            next.hand = Some(code.clone());
            self.run(&with_hand, frames.clone(), next, out);
        }
    }

    /// The places of the board holding a piece that passes `filter`, in board order.
    fn pieces_matching(&self, filter: &Filter, base: &Pieces, run: &mut Run) -> Vec<Position> {
        self.materialise(base, run);
        let pieces = run.after.as_ref().unwrap_or(base);
        let board = &self.game.board;
        let mut places: Vec<_> = pieces
            .iter()
            .filter(|(_, piece)| matches(filter, piece, self.player))
            .filter_map(|(place, _)| board.node(place).map(|node| (node, place.clone())))
            .collect();
        places.sort_unstable_by_key(|(node, _)| *node);
        places.into_iter().map(|(_, place)| place).collect()
    }

    /// Each move of the piece on `START` with each place it reaches, in spec order.
    fn moves_of_start(&self, base: &Pieces, run: &mut Run) -> Vec<(&'g [Step], Position)> {
        self.materialise(base, run);
        let pieces = run.after.as_ref().unwrap_or(base);
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
            let steps = movement.steps.as_ref().and_then(|by| by.get(self.player));
            let Some(steps) = steps else {
                continue;
            };
            let targets = movement.candidate_targets(self.player, start, pieces, self.game);
            moves.extend(targets.into_iter().map(|target| (steps.as_slice(), target)));
        }
        moves
    }
}

/// Whether judging `test` looks at the pieces (an `IN_ZONE` looks only at a place).
fn reads_pieces(test: &Test) -> bool {
    match test {
        Test::All(tests) | Test::Any(tests) => tests.iter().any(reads_pieces),
        Test::Not(test) => reads_pieces(test),
        Test::Count { .. } | Test::Attacked { .. } => true,
        Test::InZone { .. } => false,
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
            let Some(steps) = action.steps.get(player) else {
                continue;
            };
            let first = ways.len();
            let runner = Runner::new(self, &action.name);
            runner.run(
                &self.state.pieces,
                vec![steps.as_slice()],
                Run::default(),
                &mut ways,
            );
            keep_first_of_equal_answers(&mut ways, first);
        }
        ways
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
            let Some(steps) = movement.steps.as_ref().and_then(|by| by.get(player)) else {
                continue;
            };
            let targets = movement.candidate_targets(player, from, &self.state.pieces, self);
            if !targets.contains(target) {
                continue;
            }
            let run = Run {
                answers: vec![Answer::Place(from.clone()), Answer::Place(target.clone())],
                kinds: vec![ChoiceKind::Piece, ChoiceKind::Move],
                start: Some(from.clone()),
                target: Some(target.clone()),
                picked: Some(from.clone()),
                ..Run::default()
            };
            runner.run(&self.state.pieces, vec![steps.as_slice()], run, &mut ways);
        }
        ways
    }
}
