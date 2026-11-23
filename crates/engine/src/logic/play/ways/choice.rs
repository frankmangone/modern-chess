//! What the player to move can answer next, read from the ways the answers so far leave.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::{answered, Answer, ChoiceKind, Way};
use crate::logic::{Game, GamePhase};
use crate::shared::Position;

/// The choice in front of the player to move.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    /// The answers given so far this turn.
    pub chosen: Vec<Answer>,
    /// The chosen piece's place, drawn as selected.
    #[serde(with = "super::place_name::option")]
    pub start: Option<Position>,
    /// Each place that answers the next choice, with the kind of choice it answers.
    pub places: Vec<PlaceOption>,
    /// The codes that answer the next choice (a promotion).
    pub codes: Vec<String>,
    /// The pieces held in a bag that answer the next choice.
    pub held: Vec<String>,
    /// The actions that need no choice right now.
    pub actions: Vec<ActionOption>,
}

/// A place that answers the next choice.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlaceOption {
    #[serde(with = "super::place_name")]
    pub name: Position,
    /// What the place answers: `PIECE`, `MOVE` or `PLACE`.
    #[serde(rename = "for")]
    pub r#for: ChoiceKind,
}

/// An action the player may name now.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionOption {
    pub name: String,
    /// Whether naming it plays it at once (a pass), or only narrows the ways to it.
    pub plays: bool,
}

impl Game {
    /// The answers given so far this turn; none once the game is over.
    pub(crate) fn chosen(&self) -> &[Answer] {
        match &self.state.phase {
            GamePhase::Choosing { chosen } => chosen,
            GamePhase::GameOver { .. } => &[],
        }
    }

    /// The turn's ways that `given` leads to, in order.
    pub(crate) fn ways_following<'a>(
        &'a self,
        given: &'a [Answer],
    ) -> impl Iterator<Item = &'a Way> + 'a {
        self.state
            .ways
            .iter()
            .flatten()
            .filter(move |way| way.follows(given))
    }

    /// What the player to move can answer next:
    /// - `places`, `codes` and `hand`: the next answers of the remaining ways;
    /// - `actions`: the actions with a way that asks nothing more (they play at once), and,
    ///   when more than one action remains, every remaining action.
    pub fn choice(&self) -> Choice {
        let chosen = self.chosen();
        let next = answered(chosen);
        let mut choice = Choice {
            chosen: chosen.to_vec(),
            ..Choice::default()
        };
        let mut seen_places: HashSet<(&Position, ChoiceKind)> = HashSet::new();
        let mut actions: Vec<ActionOption> = Vec::new();
        for way in self.ways_following(chosen) {
            if choice.start.is_none() {
                choice.start = start_of(way, next);
            }
            match actions.iter_mut().find(|option| option.name == way.action) {
                Some(option) => option.plays |= way.answers.len() == next,
                None => actions.push(ActionOption {
                    name: way.action.clone(),
                    plays: way.answers.len() == next,
                }),
            }
            let (Some(answer), Some(kind)) = (way.answers.get(next), way.kinds.get(next)) else {
                continue;
            };
            match answer {
                Answer::Place(place) => {
                    if seen_places.insert((place, *kind)) {
                        choice.places.push(PlaceOption {
                            name: *place,
                            r#for: *kind,
                        });
                    }
                }
                Answer::Code(code) => push_new(&mut choice.codes, code),
                Answer::Held(code) => push_new(&mut choice.held, code),
                Answer::Action(_) => {}
            }
        }
        let several = actions.len() > 1;
        choice.actions = actions
            .into_iter()
            .filter(|option| option.plays || several)
            .collect();
        choice
    }
}

/// The place of the piece `way` picked among its first `given` answers.
fn start_of(way: &Way, given: usize) -> Option<Position> {
    way.kinds
        .iter()
        .zip(&way.answers)
        .take(given)
        .find_map(|(kind, answer)| match (kind, answer) {
            (ChoiceKind::Piece, Answer::Place(place)) => Some(*place),
            _ => None,
        })
}

fn push_new(list: &mut Vec<String>, item: &str) {
    if !list.iter().any(|known| known == item) {
        list.push(item.to_string());
    }
}
