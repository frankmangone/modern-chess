//! The ways of taking an action: each complete list of answers a player can give on their
//! turn, with the changes it makes. They are worked out once per turn, and answers only
//! filter them.

mod choice;
mod run;

use crate::logic::blueprint::test_block::{judge, TestContext};
use crate::logic::game::board_changes::simulate_from;
use crate::logic::Piece;

pub use choice::{ActionOption, Choice, PlaceOption};

use serde::{Deserialize, Serialize};

use crate::logic::{Game, GamePhase};
use crate::shared::{BoardChange, Position};

/// An answer to a choice: a place, a code, a piece in the hand, or an action's name. Written
/// `{ "place": "4,2" }`, `{ "code": "QUEEN" }`, `{ "hand": "PAWN" }` or `{ "action": "PASS" }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Answer {
    Place(#[serde(with = "place_name")] Position),
    Code(String),
    Hand(String),
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
    Hand,
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
    pub from_hand: Option<String>,
    /// Whether it removes a piece of another player.
    pub captures: bool,
    /// The pieces it takes off the board, as they stood, in the order taken.
    pub removed: Vec<Piece>,
    /// Whether the turn ends with it, even where a continuation would follow.
    pub ends_turn: bool,
    /// Where the picked piece ended, if it is still on the board.
    pub lands: Option<Position>,
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
    /// apply to every move: liberty captures and repeated positions (`settle`), `must`,
    /// `mandatory_capture` and the jump lock (check is part of each action's steps). Each way keeps its action's
    /// name, so the caller can narrow by it.
    pub(crate) fn compute_ways(&self) -> Vec<Way> {
        if matches!(self.state.phase, GamePhase::GameOver { .. }) {
            return Vec::new();
        }
        let mut ways: Vec<Way> = self
            .run_actions()
            .into_iter()
            .filter_map(|way| self.settle(way))
            .collect();
        self.narrow_by_must(&mut ways);
        if self.rules.mandatory_capture && ways.iter().any(|way| way.captures) {
            ways.retain(|way| way.captures);
        }
        // A jump in progress: only the jumping piece's captures.
        if let Some(position) = &self.state.continuing {
            let piece = Answer::Place(position.clone());
            ways.retain(|way| {
                way.captures
                    && way.kinds.first() == Some(&ChoiceKind::Piece)
                    && way.answers.first() == Some(&piece)
            });
        }
        ways
    }

    /// Narrows `ways` by each `must` test in order: if some way passes it, only those stay; if
    /// none does, nothing changes. A test is judged on the position the way leaves, which is
    /// built only when a test reads the pieces.
    pub(crate) fn narrow_by_must(&self, ways: &mut Vec<Way>) {
        let player = self.current_player_ref();
        let nowhere = Position::default();
        let after: Vec<Option<std::collections::HashMap<Position, Piece>>> =
            if self.rules.must.iter().any(run::reads_pieces) {
                ways.iter()
                    .map(|way| Some(simulate_from(&self.state.pieces, &way.changes)))
                    .collect()
            } else {
                vec![None; ways.len()]
            };
        for test in &self.rules.must {
            let passes: Vec<bool> = ways
                .iter()
                .zip(&after)
                .map(|(way, after)| {
                    judge(
                        test,
                        &TestContext {
                            game: self,
                            pieces: after.as_ref().unwrap_or(&self.state.pieces),
                            player,
                            start: None,
                            target: &nowhere,
                            each: None,
                            removed: &way.removed,
                        },
                    )
                })
                .collect();
            if passes.iter().any(|&passed| passed) {
                let mut passes = passes.iter();
                ways.retain(|_| *passes.next().unwrap_or(&false));
            }
        }
    }

    /// Works out the ways of the turn being played, from the board as it stands, and keeps
    /// them for the answers to filter.
    pub fn work_out_ways(&mut self) {
        self.state.ways = Some(self.compute_ways());
    }
}

/// A place written by its name (`"4,2"`), as answers are, rather than as coordinates.
mod place_name {
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
