//! Chance through the session: the seed is saved with the match and never shown, a saved match
//! resumes on the next number, and a restart is dealt another seed.

use modern_chess_session::{Answer, Session, View};
use serde_json::Value;

const RACE: &str = include_str!("fixtures/dice_race.json");
const CHESS: &str = include_str!("../../engine/specs/chess.json");

fn run(session: &mut Session) {
    let view = session.view();
    let runner = view
        .pieces
        .iter()
        .find(|piece| piece.player == view.turn && piece.code == "RUNNER")
        .expect("the mover has a runner")
        .position
        .clone();
    session
        .choose(Answer::Place(runner.as_str().into()))
        .expect("the runner runs");
}

fn pips(view: &View) -> u32 {
    view.bags
        .iter()
        .find(|bag| bag.name == "TRAY")
        .and_then(|bag| bag.pieces.get("PIP"))
        .copied()
        .unwrap_or(0)
}

#[test]
fn a_roll_shows_in_the_tray_and_the_seed_never_shows() {
    let mut session = Session::new_with_seed(RACE, 123456789).unwrap();
    run(&mut session);
    let view = session.view();
    assert!((1..=6).contains(&pips(&view)));
    let shown = serde_json::to_string(&view).unwrap();
    assert!(!shown.contains("seed") && !shown.contains("draws"), "{shown}");
    assert!(!shown.contains("123456789"), "{shown}");
    // The choice the mover is shown is the same whatever the next roll will be.
    let state = session.state();
    assert_eq!(state["seed"], 123456789);
    assert_eq!(state["draws"], 1);
}

#[test]
fn the_server_path_resumes_a_match_on_the_next_number() {
    let mut straight = Session::new_with_seed(RACE, 42).unwrap();
    let mut stored = Session::new_with_seed(RACE, 42).unwrap();
    for turn in 0..6 {
        run(&mut straight);
        // As the game server does on every request: save, throw the session away, resume.
        let state = stored_after(&mut stored, turn);
        stored = Session::resume(RACE, &state, turn as u64 + 1).unwrap();
        assert_eq!(straight.view(), stored.view(), "after turn {}", turn + 1);
    }
    assert_eq!(straight.state(), stored.state());
}

fn stored_after(session: &mut Session, _turn: usize) -> Value {
    run(session);
    session.state()
}

#[test]
fn a_game_with_no_chance_keeps_no_seed() {
    let session = Session::new_with_seed(CHESS, 5).unwrap();
    let state = session.state();
    assert!(state.get("seed").is_none() && state.get("draws").is_none());
}

#[test]
fn a_restart_is_dealt_another_seed_that_follows_from_the_first() {
    let mut first = Session::new_with_seed(RACE, 9).unwrap();
    run(&mut first);
    first.reset();
    let seed_after = first.state()["seed"].clone();
    assert_ne!(seed_after, Value::from(9));
    assert_eq!(first.state().get("draws"), None, "the new game has used nothing");
    let mut second = Session::new_with_seed(RACE, 9).unwrap();
    second.reset();
    assert_eq!(second.state()["seed"], seed_after, "worked out from the old seed");
}

#[test]
fn a_session_without_a_seed_still_plays_and_stays_without_one_after_a_reset() {
    let mut session = Session::new(RACE).unwrap();
    run(&mut session);
    assert!(session.state().get("seed").is_none());
    session.reset();
    assert!(session.state().get("seed").is_none());
}
