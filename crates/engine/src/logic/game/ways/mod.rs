//! The ways of taking an action: each complete list of answers a player can give on their
//! turn, with the changes it makes. They are worked out once per turn, and answers only
//! filter them.

mod legacy;
mod run;

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
    /// Whether the turn ends with it, even where a continuation would follow.
    pub ends_turn: bool,
    /// Where the picked piece ended, if it is still on the board.
    pub lands: Option<Position>,
}

impl Game {
    /// Every complete way of taking an action for the player to move, after the rules that
    /// apply to every move: liberty captures and repeated positions (`settle`), leader safety,
    /// `no_opponent_check`, `mandatory_capture` and the jump lock. Each way keeps its action's
    /// name, so the caller can narrow by it.
    // Only the tests read ways until the turn protocol plays them.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn compute_ways(&self) -> Vec<Way> {
        if matches!(self.state.phase, GamePhase::GameOver { .. }) {
            return Vec::new();
        }
        let ways = if self.rules.actions.is_empty() {
            self.legacy_ways()
        } else {
            self.run_actions()
        };
        let mut ways: Vec<Way> = ways
            .into_iter()
            .filter_map(|way| self.settle(way))
            .collect();
        self.retain_leader_safe(&mut ways);
        self.retain_no_opponent_check(&mut ways);
        if self.rules.mandatory_capture && ways.iter().any(|way| way.captures) {
            ways.retain(|way| way.captures);
        }
        // A jump in progress: only the jumping piece's captures.
        if let GamePhase::Moving {
            position,
            locked: true,
            ..
        } = &self.state.phase
        {
            let piece = Answer::Place(position.clone());
            ways.retain(|way| {
                way.captures
                    && way.kinds.first() == Some(&ChoiceKind::Piece)
                    && way.answers.first() == Some(&piece)
            });
        }
        ways
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
}
