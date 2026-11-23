mod support;

use modern_chess_session::session::Session;
use modern_chess_session::view::Status;
use modern_chess_session::Answer;
use serde_json::json;
use support::{action, at, code, destinations, hand, play};

const CHESS: &str = include_str!("../../engine/specs/chess.json");
const SHOGI: &str = include_str!("../../engine/specs/shogi.json");
const GO: &str = include_str!("../../engine/specs/go.json");
const PROMOTE: &str = include_str!("fixtures/promote.json");
const BARE: &str = include_str!("fixtures/bare.json");
const JUMP: &str = include_str!("fixtures/jump.json");

fn json(session: &Session) -> String {
    serde_json::to_string(&session.view()).unwrap()
}

fn place(name: &str) -> Answer {
    Answer::Place(name.into())
}

/// A new Shogi game in which SENTE already holds a rook.
fn shogi_with_a_rook() -> Session {
    let mut state = Session::new(SHOGI).unwrap().state();
    state["hand"] = json!({ "SENTE": { "ROOK": 1 } });
    Session::resume(SHOGI, &state, 0).unwrap()
}

#[test]
fn choosing_a_pawn_lists_its_moves() {
    let mut s = Session::new(CHESS).unwrap();
    at(&mut s, "4,1").unwrap();
    let v = s.view();
    assert_eq!(v.choice.chosen, vec![place("4,1")]);
    assert_eq!(v.choice.start.as_ref().map(|p| p.as_str()), Some("4,1"));
    assert_eq!(destinations(&v), vec!["4,2".to_string(), "4,3".to_string()]);
}

#[test]
fn moving_switches_the_turn() {
    let mut s = Session::new(CHESS).unwrap();
    play(&mut s, "4,1", "4,3");
    let v = s.view();
    assert_eq!(v.turn, "BLACK");
    assert!(v.choice.chosen.is_empty());
    assert_eq!(v.choice.start, None);
    assert!(v
        .pieces
        .iter()
        .any(|p| p.position == "4,3" && p.player == "WHITE"));
}

#[test]
fn cannot_choose_the_opponents_piece() {
    let mut s = Session::new(CHESS).unwrap();
    let before = json(&s);
    assert!(at(&mut s, "4,6").is_err());
    assert_eq!(before, json(&s));
}

#[test]
fn choosing_an_empty_square_errors_and_changes_nothing() {
    let mut s = Session::new(CHESS).unwrap();
    let before = json(&s);
    assert_eq!(
        at(&mut s, "4,4"),
        Err("that is not a choice right now".to_string())
    );
    assert_eq!(before, json(&s));
}

#[test]
fn clicking_another_piece_while_a_move_is_open_chooses_it() {
    let mut s = Session::new(CHESS).unwrap();
    at(&mut s, "4,1").unwrap();

    at(&mut s, "3,1").unwrap();

    let v = s.view();
    assert_eq!(v.choice.chosen, vec![place("3,1")]);
    assert_eq!(v.choice.start.as_ref().map(|p| p.as_str()), Some("3,1"));
    assert_eq!(v.turn, "WHITE");
}

#[test]
fn clicking_a_place_that_answers_nothing_cancels() {
    let mut s = Session::new(CHESS).unwrap();
    at(&mut s, "4,1").unwrap();

    at(&mut s, "0,5").unwrap();

    let v = s.view();
    assert!(v.choice.chosen.is_empty());
    assert_eq!(v.choice.start, None);
    assert_eq!(v.turn, "WHITE");
}

#[test]
fn cancel_puts_a_picked_piece_back() {
    let mut s = Session::new(CHESS).unwrap();
    at(&mut s, "4,1").unwrap();

    s.cancel().unwrap();

    assert!(s.view().choice.chosen.is_empty());
}

#[test]
fn an_answer_that_is_no_choice_changes_nothing() {
    let mut s = Session::new(CHESS).unwrap();
    at(&mut s, "4,1").unwrap();
    let before = json(&s);

    assert!(s.choose(Answer::Code("QUEEN".to_string())).is_err());
    assert!(s.choose(Answer::Held("PAWN".to_string())).is_err());
    assert!(s.choose(Answer::Action("PASS".to_string())).is_err());

    assert_eq!(before, json(&s));
}

#[test]
fn a_drop_is_two_answers_hand_then_place() {
    let mut s = shogi_with_a_rook();
    assert_eq!(s.view().choice.held, vec!["ROOK".to_string()]);

    hand(&mut s, "ROOK").unwrap();

    let v = s.view();
    assert_eq!(v.turn, "SENTE", "the drop waits for its square");
    assert_eq!(v.choice.chosen, vec![Answer::Held("ROOK".to_string())]);
    assert!(v
        .choice
        .places
        .iter()
        .any(|p| p.name.as_str() == "4,4" && p.r#for == modern_chess_session::ChoiceKind::Place));

    at(&mut s, "4,4").unwrap();

    let v = s.view();
    assert_eq!(v.turn, "GOTE");
    assert!(support::hand_of(&v, "SENTE").is_empty());
    assert!(v
        .pieces
        .iter()
        .any(|p| p.position == "4,4" && p.code == "ROOK"));
}

#[test]
fn pass_is_an_action_answer() {
    let mut s = Session::new(GO).unwrap();
    assert!(s
        .view()
        .choice
        .actions
        .iter()
        .any(|a| a.name == "PASS" && a.plays));

    action(&mut s, "PASS").unwrap();

    let v = s.view();
    assert_eq!(v.turn, "WHITE");
    assert_eq!(v.passes, 1);
}

#[test]
fn passing_where_it_is_not_allowed_is_refused() {
    let mut s = Session::new(CHESS).unwrap();
    let before = json(&s);
    assert!(action(&mut s, "PASS").is_err());
    assert_eq!(before, json(&s));
}

#[test]
fn promotion_is_a_code_answer() {
    let mut s = Session::new(PROMOTE).unwrap();
    play(&mut s, "0,2", "0,3");
    let v = s.view();
    assert_eq!(
        v.choice.codes,
        vec!["QUEEN".to_string(), "ROOK".to_string()]
    );
    assert_eq!(
        v.turn, "WHITE",
        "turn does not advance until the player chooses"
    );
    assert!(
        at(&mut s, "0,3").is_err(),
        "cannot choose a place while a promotion is pending"
    );
    assert!(s.cancel().is_err(), "a promotion cannot be backed out of");
    assert!(code(&mut s, "PAWN").is_err());
    assert!(!s.view().choice.codes.is_empty());

    code(&mut s, "ROOK").unwrap();

    let v = s.view();
    assert!(v.choice.codes.is_empty());
    assert_eq!(v.turn, "BLACK");
    assert!(v
        .pieces
        .iter()
        .any(|p| p.position == "0,3" && p.code == "ROOK"));
}

#[test]
fn a_promotion_shows_the_pawn_on_the_last_rank() {
    let mut s = Session::new(PROMOTE).unwrap();
    play(&mut s, "0,2", "0,3");

    let v = s.view();

    assert!(v
        .pieces
        .iter()
        .any(|p| p.position == "0,3" && p.code == "PAWN"));
    assert!(v.pieces.iter().all(|p| p.position != "0,2"));
}

#[test]
fn a_code_without_a_pending_choice_errors() {
    let mut s = Session::new(PROMOTE).unwrap();
    assert!(code(&mut s, "QUEEN").is_err());
}

#[test]
fn resume_keeps_an_open_promotion() {
    let mut live = Session::new(PROMOTE).unwrap();
    play(&mut live, "0,2", "0,3");

    let mut again = Session::resume(PROMOTE, &live.state(), 2).unwrap();

    assert_eq!(json(&live), json(&again));
    assert_eq!(
        again.view().choice.codes,
        vec!["QUEEN".to_string(), "ROOK".to_string()]
    );
    assert!(again.cancel().is_err());
    code(&mut again, "QUEEN").unwrap();
    assert_eq!(again.view().turn, "BLACK");
}

#[test]
fn a_jump_keeps_the_piece_chosen() {
    let mut s = Session::new(JUMP).unwrap();
    play(&mut s, "0,0", "0,2");
    let v = s.view();
    assert_eq!(v.turn, "WHITE", "the same player keeps jumping");
    assert_eq!(v.choice.start.as_ref().map(|p| p.as_str()), Some("0,2"));
    assert_eq!(destinations(&v), vec!["0,4".to_string()]);

    // A wrong click must not drop the jump lock.
    assert!(at(&mut s, "1,2").is_err());
    assert!(s.cancel().is_err());
    assert_eq!(
        s.view().choice.start.as_ref().map(|p| p.as_str()),
        Some("0,2")
    );

    at(&mut s, "0,4").unwrap();
    let v = s.view();
    assert_eq!(v.turn, "BLACK");
    assert_eq!(v.choice.start, None);
}

#[test]
fn choosing_another_piece_mid_jump_is_refused_in_plain_words() {
    let mut s = Session::new(JUMP).unwrap();
    play(&mut s, "0,0", "0,2");
    let before = json(&s);

    let err = at(&mut s, "2,0").unwrap_err();
    assert!(err.contains("jump"), "message was: {err}");
    assert_eq!(before, json(&s), "the jump in progress must be untouched");
}

#[test]
fn game_over_is_reported_and_final() {
    let mut s = Session::new(BARE).unwrap();
    play(&mut s, "0,0", "0,1");
    assert_eq!(
        s.view().status,
        Status::Won {
            winner: "WHITE".to_string()
        }
    );
    assert!(at(&mut s, "0,1").is_err());
    assert!(at(&mut s, "0,2").is_err());
    assert!(s.cancel().is_err());
    assert_eq!(
        s.view().status,
        Status::Won {
            winner: "WHITE".to_string()
        }
    );
}
