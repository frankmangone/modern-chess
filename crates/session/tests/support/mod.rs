//! Shared by the robustness tests: a seeded dice, the engine specs, and one random step.
#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

use modern_chess_session::Session;

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

/// One move chosen from what the board offers, or a piece of garbage, and a label for it.
pub fn step(session: &mut Session, dice: &mut Dice) -> String {
    let view = session.view();
    let garbage = dice.below(10) < 3;
    let coordinate = |dice: &mut Dice| [0u8, 1, 3, 7, 8, 9, 12, 19, 40, 255][dice.below(10)];

    if garbage {
        let (x, y) = (coordinate(dice), coordinate(dice));
        let code = GARBAGE_CODES[dice.below(GARBAGE_CODES.len())];
        return match dice.below(5) {
            0 => {
                let _ = session.select(x, y);
                format!("select({x},{y})")
            }
            1 => {
                let _ = session.make_move(x, y);
                format!("make_move({x},{y})")
            }
            2 => {
                let _ = session.drop_piece(code, x, y);
                format!("drop_piece({code:?},{x},{y})")
            }
            3 => {
                let _ = session.promote(code);
                format!("promote({code:?})")
            }
            _ => {
                let _ = session.pass();
                "pass()".to_string()
            }
        };
    }

    if let Some(options) = &view.promotion {
        let code = options[dice.below(options.len())].clone();
        let _ = session.promote(&code);
        return format!("promote({code:?})");
    }
    if view.selected.is_some() && !view.destinations.is_empty() {
        let [x, y] = view.destinations[dice.below(view.destinations.len())];
        let _ = session.make_move(x, y);
        return format!("make_move({x},{y})");
    }
    let droppable: Vec<(&String, &Vec<[u8; 2]>)> = view
        .drops
        .iter()
        .filter(|(_, squares)| !squares.is_empty())
        .collect();
    if !droppable.is_empty() && dice.below(2) == 0 {
        let (code, squares) = droppable[dice.below(droppable.len())];
        let [x, y] = squares[dice.below(squares.len())];
        let _ = session.drop_piece(code, x, y);
        return format!("drop_piece({code:?},{x},{y})");
    }
    if view.can_pass && dice.below(4) == 0 {
        let _ = session.pass();
        return "pass()".to_string();
    }
    let mine: Vec<_> = view
        .pieces
        .iter()
        .filter(|p| p.player == view.turn)
        .collect();
    if mine.is_empty() {
        let _ = session.pass();
        return "pass()".to_string();
    }
    let piece = mine[dice.below(mine.len())];
    let _ = session.select(piece.x, piece.y);
    format!("select({},{})", piece.x, piece.y)
}
