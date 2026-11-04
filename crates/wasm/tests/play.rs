use modern_chess_wasm::session::Session;
use modern_chess_wasm::view::Status;

const CHESS: &str = include_str!("../../engine/specs/chess.json");
const SHOGI: &str = include_str!("../../engine/specs/shogi.json");
const PROMOTE: &str = include_str!("fixtures/promote.json");
const BARE: &str = include_str!("fixtures/bare.json");
const JUMP: &str = include_str!("fixtures/jump.json");

fn json(session: &Session) -> String {
    serde_json::to_string(&session.view()).unwrap()
}

#[test]
fn selecting_a_pawn_lists_its_moves() {
    let mut s = Session::new(CHESS).unwrap();
    s.select(4, 1).unwrap();
    let v = s.view();
    assert_eq!(v.selected, Some([4, 1]));
    assert_eq!(v.destinations, vec![[4, 2], [4, 3]]);
}

#[test]
fn moving_switches_the_turn() {
    let mut s = Session::new(CHESS).unwrap();
    s.select(4, 1).unwrap();
    s.make_move(4, 3).unwrap();
    let v = s.view();
    assert_eq!(v.turn, "BLACK");
    assert_eq!(v.selected, None);
    assert!(v.pieces.iter().any(|p| p.x == 4 && p.y == 3 && p.player == "WHITE"));
}

#[test]
fn cannot_select_the_opponents_piece() {
    let mut s = Session::new(CHESS).unwrap();
    let before = json(&s);
    assert!(s.select(4, 6).is_err());
    assert_eq!(before, json(&s));
}

#[test]
fn selecting_an_empty_square_errors_and_changes_nothing() {
    let mut s = Session::new(CHESS).unwrap();
    let before = json(&s);
    assert!(s.select(4, 4).is_err());
    assert_eq!(before, json(&s));
}

#[test]
fn cannot_move_to_a_non_destination() {
    let mut s = Session::new(CHESS).unwrap();
    s.select(4, 1).unwrap();
    let before = json(&s);
    assert!(s.make_move(0, 5).is_err());
    assert_eq!(before, json(&s));
    assert_eq!(s.view().selected, Some([4, 1]));
}

#[test]
fn promotion_flow() {
    let mut s = Session::new(PROMOTE).unwrap();
    s.select(0, 2).unwrap();
    s.make_move(0, 3).unwrap();
    let v = s.view();
    assert_eq!(v.promotion, Some(vec!["QUEEN".to_string(), "ROOK".to_string()]));
    assert_eq!(v.turn, "WHITE", "turn does not advance until the player chooses");
    assert!(s.select(0, 3).is_err(), "cannot select while a promotion is pending");

    s.promote("ROOK").unwrap();
    let v = s.view();
    assert_eq!(v.promotion, None);
    assert_eq!(v.turn, "BLACK");
    assert!(v.pieces.iter().any(|p| p.x == 0 && p.y == 3 && p.code == "ROOK"));
}

#[test]
fn promote_rejects_unknown_option() {
    let mut s = Session::new(PROMOTE).unwrap();
    s.select(0, 2).unwrap();
    s.make_move(0, 3).unwrap();
    assert!(s.promote("PAWN").is_err());
    assert!(s.view().promotion.is_some());
}

#[test]
fn promote_without_a_pending_choice_errors() {
    let mut s = Session::new(PROMOTE).unwrap();
    assert!(s.promote("QUEEN").is_err());
}

#[test]
fn jump_keeps_the_piece_selected() {
    let mut s = Session::new(JUMP).unwrap();
    s.select(0, 0).unwrap();
    s.make_move(0, 2).unwrap();
    let v = s.view();
    assert_eq!(v.turn, "WHITE", "the same player keeps jumping");
    assert_eq!(v.selected, Some([0, 2]));
    assert_eq!(v.destinations, vec![[0, 4]]);

    // A wrong click must not drop the jump lock.
    assert!(s.make_move(1, 2).is_err());
    assert_eq!(s.view().selected, Some([0, 2]));

    s.make_move(0, 4).unwrap();
    let v = s.view();
    assert_eq!(v.turn, "BLACK");
    assert_eq!(v.selected, None);
}

#[test]
fn game_over_is_reported_and_final() {
    let mut s = Session::new(BARE).unwrap();
    s.select(0, 0).unwrap();
    s.make_move(0, 1).unwrap();
    assert_eq!(s.view().status, Status::Won { winner: "WHITE".to_string() });
    assert!(s.select(0, 1).is_err());
    assert!(s.make_move(0, 2).is_err());
    assert_eq!(s.view().status, Status::Won { winner: "WHITE".to_string() });
}

#[test]
fn hand_games_are_read_only() {
    let mut s = Session::new(SHOGI).unwrap();
    let err = s.select(0, 0).unwrap_err();
    assert!(err.contains("drops"), "message was: {err}");
    assert!(s.make_move(0, 1).is_err());
}
