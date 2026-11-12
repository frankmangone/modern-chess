//! Whatever a player sends, the engine must answer with an error or a move, never panic, and
//! the game it leaves behind must still draw and still resume from its saved state (the game
//! server does both on every request).
mod support;

use std::panic::{catch_unwind, AssertUnwindSafe};

use modern_chess_session::{Session, Status};
use support::{specs, step, Dice};

const SEEDS: u64 = 3;
const STEPS: usize = 100;

/// Plays one seeded game. Returns whether any piece ever moved, so the test can tell a game
/// that was really played from one where every input was refused.
fn play(spec: &str, seed: u64) -> Result<bool, String> {
    let mut session = Session::new(spec)?;
    let start = serde_json::to_string(&session.view().pieces).unwrap();
    let moved = |session: &Session| serde_json::to_string(&session.view().pieces).unwrap() != start;
    let mut dice = Dice(seed);
    let mut trail: Vec<String> = Vec::new();
    for revision in 0..STEPS as u64 {
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let label = step(&mut session, &mut dice);
            let view = session.view();
            serde_json::to_string(&view).map_err(|e| e.to_string())?;
            let saved = session.state();
            Session::resume(spec, &saved, revision).map_err(|e| format!("resume failed: {e}"))?;
            Ok::<(String, bool), String>((label, view.status != Status::Playing))
        }));
        match outcome {
            Err(_) => return Err(format!("panicked after {:?}", trail)),
            Ok(Err(problem)) => return Err(format!("{problem} after {:?}", trail)),
            Ok(Ok((label, over))) => {
                trail.push(label);
                if over {
                    return Ok(moved(&session));
                }
            }
        }
    }
    Ok(moved(&session))
}

#[test]
fn no_input_makes_the_engine_panic_or_leaves_an_unresumable_game() {
    let mut failures = Vec::new();
    for (name, spec) in specs() {
        let mut any_moved = false;
        for seed in 1..=SEEDS {
            match play(&spec, seed) {
                Ok(moved) => any_moved |= moved,
                Err(problem) => {
                    let tail: String = problem
                        .chars()
                        .rev()
                        .take(400)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect();
                    failures.push(format!("{name} seed {seed}: ...{tail}"));
                    any_moved = true;
                    break;
                }
            }
        }
        if !any_moved {
            failures.push(format!(
                "{name}: no seed ever moved a piece, so nothing was tested"
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
