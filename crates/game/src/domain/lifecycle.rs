//! The life of a game made by a person, as a state machine.
//!
//! One enum, one pure function ([`transition`]) that is the only place that knows which moves are
//! legal and who may make them, and a table (`MOVES`) the tests and the database trigger both follow.
//! The ownership plan (`docs/superpowers/plans/2026-11-27-ownership-and-credit.md`, "State machines")
//! is the source of the rules.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Where a person's game is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    /// Private, and the only state that can be edited.
    Draft,
    /// On the hub. Its revisions never change.
    Published,
    /// Taken off the hub, by its author or by staff. Can be published again by its author.
    Unpublished,
    /// Taken down by staff. Only staff can bring it back.
    Removed,
}

/// Who is asking for a move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Actor {
    Author,
    Staff,
}

/// What is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    Publish,
    Unpublish,
    Remove,
    Reinstate,
}

/// Why a move was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Refusal {
    #[error("a game cannot make that move from where it is")]
    NotFromHere,
    #[error("only the right person can make that move")]
    NotYours,
}

/// Every legal move: from a state, on an event, by these actors, to a state.
pub const MOVES: &[(GameStatus, Event, &[Actor], GameStatus)] = &[
    (
        GameStatus::Draft,
        Event::Publish,
        &[Actor::Author],
        GameStatus::Published,
    ),
    (
        GameStatus::Published,
        Event::Unpublish,
        &[Actor::Author, Actor::Staff],
        GameStatus::Unpublished,
    ),
    (
        GameStatus::Unpublished,
        Event::Publish,
        &[Actor::Author],
        GameStatus::Published,
    ),
    (
        GameStatus::Published,
        Event::Remove,
        &[Actor::Staff],
        GameStatus::Removed,
    ),
    (
        GameStatus::Unpublished,
        Event::Remove,
        &[Actor::Staff],
        GameStatus::Removed,
    ),
    (
        GameStatus::Removed,
        Event::Reinstate,
        &[Actor::Staff],
        GameStatus::Unpublished,
    ),
];

/// The state a game moves to, or why it may not.
pub fn transition(from: GameStatus, event: Event, actor: Actor) -> Result<GameStatus, Refusal> {
    let (_, _, who, to) = MOVES
        .iter()
        .find(|(state, on, _, _)| *state == from && *on == event)
        .ok_or(Refusal::NotFromHere)?;
    if who.contains(&actor) {
        Ok(*to)
    } else {
        Err(Refusal::NotYours)
    }
}

impl GameStatus {
    /// The text stored in the database.
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Published => "PUBLISHED",
            Self::Unpublished => "UNPUBLISHED",
            Self::Removed => "REMOVED",
        }
    }

    pub fn from_db(raw: &str) -> Option<Self> {
        Some(match raw {
            "DRAFT" => Self::Draft,
            "PUBLISHED" => Self::Published,
            "UNPUBLISHED" => Self::Unpublished,
            "REMOVED" => Self::Removed,
            _ => return None,
        })
    }

    /// Only a draft may be edited: a published game's revisions never change (decision 3).
    pub fn is_editable(self) -> bool {
        self == Self::Draft
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATES: [GameStatus; 4] = [
        GameStatus::Draft,
        GameStatus::Published,
        GameStatus::Unpublished,
        GameStatus::Removed,
    ];
    const EVENTS: [Event; 4] = [
        Event::Publish,
        Event::Unpublish,
        Event::Remove,
        Event::Reinstate,
    ];
    const ACTORS: [Actor; 2] = [Actor::Author, Actor::Staff];

    /// Every state, event and actor, written out: the one place the whole table is read at once.
    fn expected(from: GameStatus, event: Event, actor: Actor) -> Result<GameStatus, Refusal> {
        use Actor::{Author, Staff};
        use Event::{Publish, Reinstate, Remove, Unpublish};
        use GameStatus::{Draft, Published, Removed, Unpublished};
        match (from, event, actor) {
            (Draft, Publish, Author) => Ok(Published),
            (Draft, Publish, Staff) => Err(Refusal::NotYours),
            (Published, Unpublish, _) => Ok(Unpublished),
            (Unpublished, Publish, Author) => Ok(Published),
            (Unpublished, Publish, Staff) => Err(Refusal::NotYours),
            (Published | Unpublished, Remove, Staff) => Ok(Removed),
            (Published | Unpublished, Remove, Author) => Err(Refusal::NotYours),
            (Removed, Reinstate, Staff) => Ok(Unpublished),
            (Removed, Reinstate, Author) => Err(Refusal::NotYours),
            _ => Err(Refusal::NotFromHere),
        }
    }

    #[test]
    fn every_state_event_and_actor_is_answered_as_the_plan_says() {
        for from in STATES {
            for event in EVENTS {
                for actor in ACTORS {
                    assert_eq!(
                        transition(from, event, actor),
                        expected(from, event, actor),
                        "{from:?} on {event:?} by {actor:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_removed_game_can_only_be_brought_back_by_staff_and_not_published_by_its_author() {
        assert_eq!(
            transition(GameStatus::Removed, Event::Publish, Actor::Author),
            Err(Refusal::NotFromHere)
        );
        assert_eq!(
            transition(GameStatus::Removed, Event::Reinstate, Actor::Author),
            Err(Refusal::NotYours)
        );
    }

    #[test]
    fn only_a_draft_is_editable() {
        let editable: Vec<_> = STATES.into_iter().filter(|s| s.is_editable()).collect();
        assert_eq!(editable, vec![GameStatus::Draft]);
    }

    #[test]
    fn the_database_text_round_trips_and_unknown_text_is_not_a_status() {
        for status in STATES {
            assert_eq!(GameStatus::from_db(status.as_db()), Some(status));
        }
        assert_eq!(GameStatus::from_db("draft"), None);
    }
}
