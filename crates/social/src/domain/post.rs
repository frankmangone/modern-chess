//! The life of a post, as a state machine.
//!
//! One enum, one pure function ([`transition`]) that is the only place that knows which moves are
//! legal and who may make them, and a table (`MOVES`) the tests and the database trigger both
//! follow. The design is `docs/superpowers/specs/2026-11-28-discussion-design.md`.

use serde::Serialize;
use thiserror::Error;

/// Where a post is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PostStatus {
    /// Readable by everyone.
    Visible,
    /// Hidden by staff. It shows as a grey line and its words are sent to nobody. Staff can bring
    /// it back; its author cannot delete it (they could otherwise slip out of a reinstatement).
    Hidden,
    /// Deleted by its author. Shows as a grey line, and never comes back.
    Deleted,
}

impl PostStatus {
    /// How the database spells it.
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Visible => "VISIBLE",
            Self::Hidden => "HIDDEN",
            Self::Deleted => "DELETED",
        }
    }

    pub fn from_db(text: &str) -> Option<Self> {
        match text {
            "VISIBLE" => Some(Self::Visible),
            "HIDDEN" => Some(Self::Hidden),
            "DELETED" => Some(Self::Deleted),
            _ => None,
        }
    }
}

/// Who is asking for a move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Actor {
    /// The person who wrote the post (staff acting on their own post act as its author).
    Author,
    Staff,
}

impl Actor {
    pub fn as_db(self) -> &'static str {
        match self {
            Self::Author => "AUTHOR",
            Self::Staff => "STAFF",
        }
    }
}

/// What is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Event {
    Hide,
    Reinstate,
    Delete,
}

/// Why a move was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Refusal {
    #[error("a post cannot make that move from where it is")]
    NotFromHere,
    #[error("only the right person can make that move")]
    NotYours,
}

/// Every legal move: from a state, on an event, by these actors, to a state.
pub const MOVES: &[(PostStatus, Event, &[Actor], PostStatus)] = &[
    (
        PostStatus::Visible,
        Event::Hide,
        &[Actor::Staff],
        PostStatus::Hidden,
    ),
    (
        PostStatus::Hidden,
        Event::Reinstate,
        &[Actor::Staff],
        PostStatus::Visible,
    ),
    (
        PostStatus::Visible,
        Event::Delete,
        &[Actor::Author],
        PostStatus::Deleted,
    ),
];

/// The state a post moves to, or why it may not.
pub fn transition(from: PostStatus, event: Event, actor: Actor) -> Result<PostStatus, Refusal> {
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

#[cfg(test)]
mod tests {
    use super::*;

    const STATUSES: [PostStatus; 3] =
        [PostStatus::Visible, PostStatus::Hidden, PostStatus::Deleted];
    const EVENTS: [Event; 3] = [Event::Hide, Event::Reinstate, Event::Delete];
    const ACTORS: [Actor; 2] = [Actor::Author, Actor::Staff];

    /// Every state, event and actor: it is allowed exactly when the table says so, and a refusal
    /// says whether the move does not exist from there or is somebody else's.
    #[test]
    fn every_combination_is_allowed_or_refused_as_the_table_says() {
        for from in STATUSES {
            for event in EVENTS {
                for actor in ACTORS {
                    let row = MOVES.iter().find(|(s, e, _, _)| *s == from && *e == event);
                    let expected = match row {
                        None => Err(Refusal::NotFromHere),
                        Some((_, _, who, to)) if who.contains(&actor) => Ok(*to),
                        Some(_) => Err(Refusal::NotYours),
                    };
                    assert_eq!(
                        transition(from, event, actor),
                        expected,
                        "{from:?} {event:?} by {actor:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_rules_the_design_states() {
        use PostStatus::*;
        // Staff hide and reinstate; the author deletes.
        assert_eq!(transition(Visible, Event::Hide, Actor::Staff), Ok(Hidden));
        assert_eq!(
            transition(Hidden, Event::Reinstate, Actor::Staff),
            Ok(Visible)
        );
        assert_eq!(
            transition(Visible, Event::Delete, Actor::Author),
            Ok(Deleted)
        );
        // A staff-hidden post cannot be deleted by its author, and a deleted one never returns.
        assert_eq!(
            transition(Hidden, Event::Delete, Actor::Author),
            Err(Refusal::NotFromHere)
        );
        for event in EVENTS {
            for actor in ACTORS {
                assert!(transition(Deleted, event, actor).is_err());
            }
        }
        // Nobody hides their own post as staff: acting on your own, you are its author.
        assert_eq!(
            transition(Visible, Event::Hide, Actor::Author),
            Err(Refusal::NotYours)
        );
    }

    #[test]
    fn statuses_round_trip_through_the_database_spelling() {
        for status in STATUSES {
            assert_eq!(PostStatus::from_db(status.as_db()), Some(status));
        }
        assert_eq!(PostStatus::from_db("visible"), None);
    }
}
