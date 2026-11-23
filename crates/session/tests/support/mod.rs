//! Shared by the robustness tests: a seeded dice, the engine specs, and one random step.
#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

use modern_chess::logic::ChoiceKind;
use modern_chess::shared::Position;
use modern_chess_session::{Answer, Session, View};

pub const GARBAGE_CODES: [&str; 5] = ["", "zz", "P", "\u{1f4a5}", "a,b"];

/// A tiny deterministic generator, so a failure can be replayed from its seed.
pub struct Dice(pub u64);

impl Dice {
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

pub fn specs() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../engine/specs");
    let mut found: Vec<(String, String)> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.unwrap().path();
            (path.extension()? == "json").then(|| {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                (name, fs::read_to_string(&path).unwrap())
            })
        })
        .collect();
    found.sort();
    found
}

/// Chooses the place `name`, as a click on it would.
pub fn at(session: &mut Session, name: &str) -> Result<(), String> {
    session.choose(Answer::Place(Position::from(name)))
}

/// Chooses the hand piece `code`.
pub fn hand(session: &mut Session, code: &str) -> Result<(), String> {
    session.choose(Answer::Held(code.to_string()))
}

/// Chooses the code `code`, as an answer to a promotion.
pub fn code(session: &mut Session, code: &str) -> Result<(), String> {
    session.choose(Answer::Code(code.to_string()))
}

/// Names the action `name`.
pub fn action(session: &mut Session, name: &str) -> Result<(), String> {
    session.choose(Answer::Action(name.to_string()))
}

/// Picks the piece at `from` and moves it to `to`.
pub fn play(session: &mut Session, from: &str, to: &str) {
    at(session, from).unwrap();
    at(session, to).unwrap();
}

/// The places that answer the open choice as a move, in board order.
pub fn destinations(view: &View) -> Vec<String> {
    names_for(view, ChoiceKind::Move)
}

/// Every place that answers the open choice as `kind`, in board order.
pub fn names_for(view: &View, kind: ChoiceKind) -> Vec<String> {
    let mut places: Vec<&Position> = view
        .choice
        .places
        .iter()
        .filter(|place| place.r#for == kind)
        .map(|place| &place.name)
        .collect();
    places.sort();
    places.into_iter().map(|p| p.as_str().to_string()).collect()
}

/// One answer chosen from what the board offers, or a piece of garbage, and a label for it.
pub fn step(session: &mut Session, dice: &mut Dice) -> String {
    let view = session.view();
    let garbage = dice.below(10) < 3;
    let coordinate = |dice: &mut Dice| [0u8, 1, 3, 7, 8, 9, 12, 19, 40, 255][dice.below(10)];

    if garbage {
        let name = format!("{},{}", coordinate(dice), coordinate(dice));
        let text = GARBAGE_CODES[dice.below(GARBAGE_CODES.len())].to_string();
        let answer = match dice.below(6) {
            0 | 1 => Some(Answer::Place(Position::from(name.as_str()))),
            2 => Some(Answer::Code(text)),
            3 => Some(Answer::Held(text)),
            4 => Some(Answer::Action(text)),
            _ => None,
        };
        return match answer {
            Some(answer) => {
                let label = format!("choose({answer:?})");
                let _ = session.choose(answer);
                label
            }
            None => {
                let _ = session.cancel();
                "cancel()".to_string()
            }
        };
    }

    let choice = &view.choice;
    let mut offered: Vec<Answer> = Vec::new();
    offered.extend(choice.places.iter().map(|place| Answer::Place(place.name)));
    offered.extend(choice.codes.iter().cloned().map(Answer::Code));
    offered.extend(choice.held.iter().cloned().map(Answer::Held));
    offered.extend(
        choice
            .actions
            .iter()
            .map(|action| Answer::Action(action.name.clone())),
    );
    let mine: Vec<_> = view
        .pieces
        .iter()
        .filter(|p| p.player == view.turn)
        .collect();
    // Now and then, a click on one of the player's own pieces, which may cancel first.
    if !mine.is_empty() && dice.below(8) == 0 {
        let piece = mine[dice.below(mine.len())];
        let _ = at(session, &piece.position);
        return format!("click({})", piece.position);
    }
    if offered.is_empty() || (!choice.chosen.is_empty() && dice.below(10) == 0) {
        let _ = session.cancel();
        return "cancel()".to_string();
    }
    let answer = offered.swap_remove(dice.below(offered.len()));
    let label = format!("choose({answer:?})");
    let _ = session.choose(answer);
    label
}

/// What `player` keeps in the bag `HAND`, as code -> count.
pub fn hand_of<'a>(view: &'a View, player: &str) -> &'a std::collections::BTreeMap<String, u32> {
    &view
        .bags
        .iter()
        .find(|bag| bag.name == "HAND" && bag.owner.as_deref() == Some(player))
        .expect("the player has a hand")
        .pieces
}
