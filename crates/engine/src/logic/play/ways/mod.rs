//! The ways of taking an action: each complete list of answers a player can give on their
//! turn, with the changes it makes. They are worked out once per turn, and answers only
//! filter them.

mod choice;
mod run;

use crate::logic::{PiecesView, Removal};

use crate::logic::rules::test_block::{judge, TestContext};

pub use choice::{ActionOption, Choice, PlaceOption};

use serde::{Deserialize, Serialize};

use crate::logic::places::holders::Holder;
use crate::logic::rules::steps::AskStep;
use crate::logic::{Game, GamePhase, NodeId};
use crate::shared::{BoardChange, Position, Symbol};

/// An answer to a choice: a place, a code, a piece held in a bag, or an action's name. Written
/// `{ "place": "4,2" }`, `{ "code": "QUEEN" }`, `{ "held": "PAWN" }` or `{ "action": "PASS" }`.
/// (`{ "hand": ... }`, as it was written before bags, is still read.)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Answer {
    Place(#[serde(with = "place_name")] Position),
    Code(String),
    #[serde(alias = "hand")]
    Held(String),
    Action(String),
}

/// What a choice asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ChoiceKind {
    Piece,
    Move,
    Place,
    Code,
    Held,
}

/// A change a way makes to a counter, applied when the way is played.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CountChange {
    pub counter: String,
    /// Every player's counter, not only the mover's.
    pub all: bool,
    /// Set to 0, instead of adding `by`.
    pub reset: bool,
    pub by: i64,
}

/// Pieces added to (`delta` above 0) or taken out of (below 0) a holder, or, for a pile, its whole
/// new contents (`contents`; the other fields then mean nothing).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HolderChange {
    pub at: Position,
    pub owner: Symbol,
    pub code: Symbol,
    pub delta: i32,
    /// What the holder keeps once this change is made, when the change replaces it.
    pub contents: Option<Holder>,
}

impl HolderChange {
    /// A change that leaves the holder at `at` keeping `contents`.
    pub fn replacing(at: Position, contents: Holder) -> Self {
        HolderChange {
            at,
            owner: Symbol::new(""),
            code: Symbol::new(""),
            delta: 0,
            contents: Some(contents),
        }
    }

    /// Makes the change to `holder`, which is what the holder at `self.at` kept before it.
    pub fn apply_to(&self, holder: &mut Holder) {
        match &self.contents {
            Some(contents) => *holder = contents.clone(),
            None => holder.apply(self.owner, self.code, self.delta),
        }
    }
}

/// One die of a roll, as it landed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rolled {
    /// The bag it was rolled into.
    pub at: Position,
    /// The face it landed on: the pieces it put there.
    pub face: Vec<(Symbol, u32)>,
}

/// One complete way of taking an action.
#[derive(Clone, Debug)]
pub struct Way {
    /// The action's name.
    pub action: String,
    /// The place, code and hand answers, in order. Never an `Answer::Action`: the action's
    /// name is `action`.
    pub answers: Vec<Answer>,
    /// What each answer answered.
    pub kinds: Vec<ChoiceKind>,
    /// The changes it makes, in order. Every piece it takes off the board is a removal here.
    pub changes: Vec<BoardChange>,
    /// The code of the piece that leaves the mover's hand, if one does.
    pub from_hand: Option<Symbol>,
    /// What it adds to and takes out of holders (bags and places that hold many), in order.
    pub holder_changes: Vec<HolderChange>,
    /// Whether it removes a piece of another player.
    pub captures: bool,
    /// The pieces it takes off the board, as they stood, in the order taken.
    pub removed: Vec<Removal>,
    /// The pieces of the mover that it puts into the mover's own bag, or a place that holds many,
    /// off the board, as `(player, code)`: kept, not lost, so no capture.
    pub stored: Vec<(Symbol, Symbol)>,
    /// The places it moves a piece from, along the board; the pieces are read as they stood
    /// when the turn began.
    pub moved: Vec<NodeId>,
    /// The changes it makes to counters, in order.
    pub counts: Vec<CountChange>,
    /// The place the first `PIECE` or `HELD` choice named: `START`, if the way has one.
    pub start: Option<Position>,
    /// Where its latest `MOVE` or `PLACE` put a piece: `LAST`.
    pub last: Option<Position>,
    /// The stage it puts the turn in, if it runs `STAGE`.
    pub stage: Option<Symbol>,
    /// The seats it asks, if it runs `ASK`.
    pub ask: Option<AskStep>,
    /// The bags its `HELD` choices took from, in order.
    pub held_bags: Vec<Position>,
    /// How many random numbers it uses up, if played.
    pub draws: u32,
    /// The dice it rolls, in order.
    pub rolled: Vec<Rolled>,
    /// The piles it shuffles, in order.
    pub shuffled: Vec<Position>,
    /// The answers given once some of the changes were made, each with how many: `(index of
    /// the answer, changes made before it)`, in order. An answer that is not here came before
    /// any change. A promotion's code comes after the move.
    pub effects_before: Vec<(usize, usize)>,
}

impl Way {
    /// Whether `given` (answers so far, action names among them) leads to this way: each
    /// action name is its action's, and the other answers start its answers.
    pub fn follows(&self, given: &[Answer]) -> bool {
        let mut answers = self.answers.iter();
        given.iter().all(|answer| match answer {
            Answer::Action(name) => *name == self.action,
            answer => answers.next() == Some(answer),
        })
    }

    /// How many of the changes are made before the answer at `index` is given; all of them
    /// when the way asks nothing more.
    pub(crate) fn changes_before(&self, index: usize) -> usize {
        if index >= self.answers.len() {
            return self.changes.len();
        }
        self.effects_before
            .iter()
            .take_while(|(answer, _)| *answer <= index)
            .last()
            .map_or(0, |(_, made)| *made)
    }
}

/// How many of `given` are answers to choices, not action names.
pub(crate) fn answered(given: &[Answer]) -> usize {
    given
        .iter()
        .filter(|answer| !matches!(answer, Answer::Action(_)))
        .count()
}

impl Game {
    /// Every complete way of taking an action for the player to move, after the rules that
    /// apply to every move: the jump lock and `must` (check, liberties and repeated positions
    /// are part of each action's steps). Each way keeps its action's name, so the caller can
    /// narrow by it.
    pub(crate) fn compute_ways(&self) -> Vec<Way> {
        if matches!(self.state.phase, GamePhase::GameOver { .. }) {
            return Vec::new();
        }
        // While every seat is asked at once nobody is the player to move: only a seat has ways.
        if self.state.together.is_some() && self.state.focus.is_none() {
            return Vec::new();
        }
        let mut ways: Vec<Way> = self.run_actions();
        // A stage in progress: only the ways its `only` allows. This comes before `must`, so
        // an entry that only another way could meet cannot empty the stage.
        self.narrow_by_stage(&mut ways);
        self.narrow_by_must(&mut ways);
        ways
    }

    /// Keeps the ways the stage being played allows: those that pass its `only`, judged on the
    /// position each leaves, with `START` the way's piece and `LAST` where the way before ended.
    fn narrow_by_stage(&self, ways: &mut Vec<Way>) {
        let Some(stage) = &self.state.stage else {
            return;
        };
        let Some(rule) = self
            .rules
            .stages
            .iter()
            .find(|rule| rule.name == stage.name)
        else {
            return;
        };
        let player = self.current_player_ref();
        // A player the stage is not for has no way in it.
        if !rule.players.is_empty() && !rule.players.iter().any(|name| name == player) {
            ways.clear();
            return;
        }
        let Some(only) = &rule.only else {
            return;
        };
        let nowhere = Position::default();
        ways.retain(|way| {
            judge(
                only,
                &TestContext {
                    game: self,
                    pieces: PiecesView::with(&self.state.pieces, &way.changes)
                        .with_holders(&self.state.holders, &way.holder_changes),
                    player,
                    start: way.start.as_ref(),
                    target: &nowhere,
                    each: None,
                    removed: &way.removed,
                    moved: &way.moved,
                    after_turn: None,
                    last: stage.last.as_ref(),
                },
            )
        });
    }

    /// Narrows `ways` by each `must` test in order: if some way passes it, only those stay; if
    /// none does, nothing changes. A test is judged on the position the way leaves, which is
    /// built only when a test reads the pieces, and travels with its way as the ways narrow.
    pub(crate) fn narrow_by_must(&self, ways: &mut Vec<Way>) {
        if self.rules.must.is_empty() {
            return;
        }
        let player = self.current_player_ref();
        let nowhere = Position::default();
        let mut judged: Vec<Way> = std::mem::take(ways);
        for test in &self.rules.must {
            let passes: Vec<bool> = judged
                .iter()
                .map(|way| {
                    judge(
                        test,
                        &TestContext {
                            game: self,
                            pieces: PiecesView::with(&self.state.pieces, &way.changes)
                                .with_holders(&self.state.holders, &way.holder_changes),
                            player,
                            start: None,
                            target: &nowhere,
                            each: None,
                            removed: &way.removed,
                            moved: &way.moved,
                            after_turn: None,
                            last: None,
                        },
                    )
                })
                .collect();
            if passes.iter().any(|&passed| passed) {
                let mut passes = passes.iter();
                judged.retain(|_| *passes.next().unwrap_or(&false));
            }
        }
        ways.extend(judged);
    }

    /// Works out the ways of the turn being played, from the board as it stands, and keeps
    /// them for the answers to filter.
    pub fn work_out_ways(&mut self) {
        self.state.ways = Some(self.compute_ways());
    }
}

/// A place written by its name (`"4,2"`), as answers are, rather than as coordinates.
pub(crate) mod place_name {
    use serde::{Deserialize, Deserializer, Serializer};

    use crate::shared::Position;

    pub fn serialize<S: Serializer>(place: &Position, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(place.as_str())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Position, D::Error> {
        String::deserialize(deserializer).map(Position::from)
    }

    /// The same for a place that may be missing, written `null`.
    pub mod option {
        use serde::{Deserialize, Deserializer, Serializer};

        use crate::shared::Position;

        pub fn serialize<S: Serializer>(
            place: &Option<Position>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            match place {
                Some(place) => serializer.serialize_some(place.as_str()),
                None => serializer.serialize_none(),
            }
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Option<Position>, D::Error> {
            Option::<String>::deserialize(deserializer).map(|name| name.map(Position::from))
        }
    }
}

impl Game {
    /// Whether the player to move has at least one way of taking an action, worked out from
    /// the board as it stands. A game with no ways has no legal move.
    pub fn any_legal_moves(&self) -> bool {
        !self.compute_ways().is_empty()
    }
}
