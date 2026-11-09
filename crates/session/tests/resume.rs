//! A session saved and resumed (what the game server does on every request) must show exactly
//! what the live session showed.
use modern_chess_session::Session;

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
    live.select(4, 1).unwrap();
    live.make_move(4, 3).unwrap();
    live.select(4, 6).unwrap(); // black has a piece picked up: moves must be recomputed

    let again = resumed(CHESS, &live, 3);

    assert_eq!(json(&live), json(&again));
    assert_eq!(again.view().destinations, vec![[4, 4], [4, 5]]);
    assert_eq!(again.revision(), 3);
}

#[test]
fn a_resumed_session_keeps_playing_like_the_live_one() {
    let mut live = Session::new(CHESS).unwrap();
    live.select(4, 1).unwrap();
    let mut again = resumed(CHESS, &live, 1);

    live.make_move(4, 3).unwrap();
    again.make_move(4, 3).unwrap();

    assert_eq!(json(&live), json(&again));
    assert_eq!(again.view().turn, "BLACK");
}

#[test]
fn resume_keeps_a_pending_promotion() {
    let mut live = Session::new(PROMOTE).unwrap();
    live.select(0, 2).unwrap();
    live.make_move(0, 3).unwrap();

    let mut again = resumed(PROMOTE, &live, 2);

    assert_eq!(json(&live), json(&again));
    assert!(again.view().promotion.is_some());
    again.promote("QUEEN").unwrap();
    assert_eq!(again.view().turn, "BLACK");
}

#[test]
fn resume_keeps_a_jump_in_progress() {
    let mut live = Session::new(JUMP).unwrap();
    live.select(0, 0).unwrap();
    live.make_move(0, 2).unwrap();

    let mut again = resumed(JUMP, &live, 2);

    assert_eq!(json(&live), json(&again));
    assert_eq!(again.view().selected, Some([0, 2]));
    assert_eq!(again.view().destinations, vec![[0, 4]]);
    let err = again.select(2, 0).unwrap_err();
    assert!(err.contains("jump"), "the lock survived: {err}");
}

#[test]
fn resume_keeps_a_finished_game_final() {
    let mut live = Session::new(BARE).unwrap();
    live.select(0, 0).unwrap();
    live.make_move(0, 1).unwrap();

    let mut again = resumed(BARE, &live, 2);

    assert_eq!(json(&live), json(&again));
    assert!(again.select(0, 0).is_err());
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
