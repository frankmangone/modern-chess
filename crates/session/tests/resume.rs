//! A session saved and resumed (what the game server does on every request) must show exactly
//! what the live session showed.
mod support;

use modern_chess_session::Session;
use support::{at, code, destinations, play};

const CHESS: &str = include_str!("../../engine/specs/chess.json");
const PROMOTE: &str = include_str!("fixtures/promote.json");
const BARE: &str = include_str!("fixtures/bare.json");
const JUMP: &str = include_str!("fixtures/jump.json");

fn json(session: &Session) -> String {
    serde_json::to_string(&session.view()).unwrap()
}

fn resumed(spec: &str, live: &Session, revision: u64) -> Session {
    Session::resume(spec, &live.state(), revision).expect("a saved state resumes")
}

#[test]
fn session_resume_reproduces_the_same_view() {
    let mut live = Session::new(CHESS).unwrap();
    play(&mut live, "4,1", "4,3");
    at(&mut live, "4,6").unwrap(); // black has a piece picked up: moves must be recomputed

    let again = resumed(CHESS, &live, 3);

    assert_eq!(json(&live), json(&again));
    assert_eq!(
        destinations(&again.view()),
        vec!["4,4".to_string(), "4,5".to_string()]
    );
    assert_eq!(again.revision(), 3);
}

#[test]
fn a_resumed_session_keeps_playing_like_the_live_one() {
    let mut live = Session::new(CHESS).unwrap();
    at(&mut live, "4,1").unwrap();
    let mut again = resumed(CHESS, &live, 1);

    at(&mut live, "4,3").unwrap();
    at(&mut again, "4,3").unwrap();

    assert_eq!(json(&live), json(&again));
    assert_eq!(again.view().turn, "BLACK");
}

#[test]
fn resume_keeps_an_open_promotion_and_finishes_it() {
    let mut live = Session::new(PROMOTE).unwrap();
    play(&mut live, "0,2", "0,3");

    let mut again = resumed(PROMOTE, &live, 2);

    assert_eq!(json(&live), json(&again));
    assert!(!again.view().choice.codes.is_empty());
    code(&mut again, "QUEEN").unwrap();
    assert_eq!(again.view().turn, "BLACK");
}

#[test]
fn resume_keeps_a_jump_in_progress() {
    let mut live = Session::new(JUMP).unwrap();
    play(&mut live, "0,0", "0,2");

    let mut again = resumed(JUMP, &live, 2);

    assert_eq!(json(&live), json(&again));
    let view = again.view();
    assert_eq!(view.choice.start.as_ref().map(|p| p.as_str()), Some("0,2"));
    assert_eq!(destinations(&view), vec!["0,4".to_string()]);
    let err = at(&mut again, "2,0").unwrap_err();
    assert!(err.contains("jump"), "the lock survived: {err}");
}

#[test]
fn resume_refuses_a_saved_stage_the_rules_do_not_have() {
    let mut live = Session::new(JUMP).unwrap();
    play(&mut live, "0,0", "0,2");
    let mut state = live.state();
    assert_eq!(state["stage"]["name"], "JUMPING", "the fixture is mid-jump");

    state["stage"]["name"] = "FLYING".into();
    let error = Session::resume(JUMP, &state, 2)
        .err()
        .expect("an unknown stage is refused");
    assert!(error.contains("does not fit"), "{error}");
}

#[test]
fn resume_refuses_a_saved_stage_that_leaves_the_turn_no_way() {
    let mut live = Session::new(JUMP).unwrap();
    play(&mut live, "0,0", "0,2");
    let mut state = live.state();

    // The jumping piece is gone from where the stage's `LAST` says it ended.
    state["stage"]["last"] = "5,5".into();
    let error = Session::resume(JUMP, &state, 2)
        .err()
        .expect("no way to play is refused");
    assert!(error.contains("does not fit"), "{error}");
}

#[test]
fn resume_keeps_a_finished_game_final() {
    let mut live = Session::new(BARE).unwrap();
    play(&mut live, "0,0", "0,1");

    let mut again = resumed(BARE, &live, 2);

    assert_eq!(json(&live), json(&again));
    assert!(at(&mut again, "0,0").is_err());
}

#[test]
fn resume_refuses_a_state_that_is_not_a_game_state() {
    let nonsense = serde_json::json!({ "pieces": "none" });
    assert!(Session::resume(CHESS, &nonsense, 0).is_err());
}

#[test]
fn resume_refuses_rules_that_do_not_parse() {
    let live = Session::new(CHESS).unwrap();
    assert!(Session::resume("{}", &live.state(), 0).is_err());
}

#[test]
fn a_saved_state_whose_turn_is_outside_the_players_is_refused() {
    let mut state = Session::new(CHESS).unwrap().state();
    state["current_turn"] = serde_json::json!(9);

    let result = Session::resume(CHESS, &state, 0);

    assert_eq!(
        result.err().as_deref(),
        Some("the saved state does not fit these rules")
    );
}

#[test]
fn a_state_saved_without_a_turn_order_resumes_with_the_spec_order() {
    let mut live = Session::new(CHESS).unwrap();
    play(&mut live, "4,1", "4,3");
    let mut state = live.state();
    state.as_object_mut().unwrap().remove("turn_order");

    let again = Session::resume(CHESS, &state, 1).expect("an older saved state resumes");

    assert_eq!(json(&live), json(&again));
}
