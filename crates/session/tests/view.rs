use modern_chess_session::session::Session;
use modern_chess_session::view::Status;

const CHESS: &str = include_str!("../../engine/specs/chess.json");
const SHOGI: &str = include_str!("../../engine/specs/shogi.json");
const MARCH: &str = include_str!("../../../apps/docs/examples/march.json");

#[test]
fn view_of_chess_start() {
    let session = Session::new(CHESS).unwrap();
    let view = session.view();
    assert_eq!(view.width, 8);
    assert_eq!(view.height, 8);
    assert_eq!(view.pieces.len(), 32);
    assert_eq!(view.turn, "WHITE");
    assert_eq!(view.status, Status::Playing);
    assert_eq!(view.selected, None);
    assert!(view.destinations.is_empty());
    assert_eq!(view.promotion, None);
    assert!(!view.hands);
}

#[test]
fn view_reports_disabled_squares() {
    let spec = MARCH.replacen(
        "\"dimensions\": [3, 5] }",
        "\"dimensions\": [3, 5], \"disabled_positions\": [[1, 2]] }",
        1,
    );
    assert_ne!(spec, MARCH);
    let view = Session::new(&spec).unwrap().view();
    assert_eq!(view.disabled, vec![[1u8, 2u8]]);
}

#[test]
fn view_reports_zones() {
    let view = Session::new(CHESS).unwrap().view();
    let zone = view.zones.get("REACH_END").expect("REACH_END zone present");
    assert!(!zone.get("WHITE").expect("WHITE squares").is_empty());
}

#[test]
fn view_flags_hand_games() {
    assert!(Session::new(SHOGI).unwrap().view().hands);
}

#[test]
fn new_rejects_bad_json() {
    assert!(Session::new("{").is_err());
}

#[test]
fn new_rejects_non_2d_board() {
    // The engine's direction matrices are 2x2, so a 3-D board cannot be described at all.
    let spec = MARCH.replacen("[3, 5]", "[3, 5, 2]", 1);
    assert_ne!(spec, MARCH);
    assert!(Session::new(&spec).is_err());
}

#[test]
fn reset_restores_start() {
    let mut session = Session::new(CHESS).unwrap();
    let before = serde_json::to_string(&session.view()).unwrap();
    session.reset();
    let after = serde_json::to_string(&session.view()).unwrap();
    assert_eq!(before, after);
}
