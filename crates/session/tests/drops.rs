//! Playing pieces from the hand, as in Shogi: what the view offers, and what a drop does.

mod support;

use modern_chess_session::session::Session;
use modern_chess_session::ChoiceKind;
use serde_json::json;
use support::{at, code, hand, names_for};

const SHOGI: &str = include_str!("../../engine/specs/shogi.json");
const CHESS: &str = include_str!("../../engine/specs/chess.json");

fn json(session: &Session) -> String {
    serde_json::to_string(&session.view()).unwrap()
}

/// A new Shogi game in which SENTE already holds these pieces.
fn shogi_with_hand(pieces: &[(&str, u32)]) -> Session {
    let mut state = Session::new(SHOGI).unwrap().state();
    let hand: serde_json::Map<String, serde_json::Value> = pieces
        .iter()
        .map(|(code, count)| (code.to_string(), json!(count)))
        .collect();
    state["hand"] = json!({ "SENTE": hand });
    Session::resume(SHOGI, &state, 0).unwrap()
}

fn drop_piece(session: &mut Session, code: &str, square: &str) {
    hand(session, code).unwrap();
    at(session, square).unwrap();
}

fn play(session: &mut Session, from: (u8, u8), to: (u8, u8)) {
    support::play(
        session,
        &format!("{},{}", from.0, from.1),
        &format!("{},{}", to.0, to.1),
    );
}

/// Both sides open the bishops' diagonal and SENTE takes GOTE's bishop, so SENTE holds one.
fn after_a_bishop_trade() -> Session {
    let mut s = Session::new(SHOGI).unwrap();
    play(&mut s, (6, 2), (6, 3)); // SENTE opens the diagonal
    play(&mut s, (2, 6), (2, 5)); // GOTE opens it too
    support::play(&mut s, "7,1", "1,7"); // takes the bishop, landing in the promotion zone
    code(&mut s, "HORSE").unwrap();
    play(&mut s, (0, 6), (0, 5)); // GOTE plays elsewhere
    s
}

#[test]
fn a_hand_game_can_be_played_on_the_board() {
    let mut s = Session::new(SHOGI).unwrap();

    support::play(&mut s, "4,2", "4,3");

    assert_eq!(s.view().turn, "GOTE");
}

#[test]
fn the_view_shows_every_hand_and_starts_empty() {
    let view = Session::new(SHOGI).unwrap().view();

    assert!(view.hands);
    assert_eq!(view.hand.len(), 2, "one entry per player");
    assert!(view.hand.values().all(|hand| hand.is_empty()));
    assert!(view.choice.hand.is_empty());
}

#[test]
fn a_game_without_a_hand_has_no_hand_in_its_view() {
    let view = Session::new(CHESS).unwrap().view();

    assert!(!view.hands);
    assert!(view.hand.is_empty());
    assert!(view.choice.hand.is_empty());
}

#[test]
fn a_capture_puts_the_piece_in_the_capturers_hand() {
    let s = after_a_bishop_trade();

    let view = s.view();

    assert_eq!(view.hand["SENTE"].get("BISHOP"), Some(&1));
    assert!(view.hand["GOTE"].is_empty());
}

/// The squares `code` may be dropped on, once it is chosen from the hand.
fn drops_of(s: &Session, code: &str) -> Vec<String> {
    let mut chosen = Session::resume(SHOGI, &s.state(), 0).unwrap();
    hand(&mut chosen, code).unwrap();
    names_for(&chosen.view(), ChoiceKind::Place)
}

#[test]
fn the_view_lists_where_each_hand_piece_may_be_dropped() {
    let s = shogi_with_hand(&[("PAWN", 1), ("KNIGHT", 1), ("LANCE", 1), ("ROOK", 1)]);

    let mut offered = s.view().choice.hand;
    offered.sort();

    // SENTE has a pawn on every file, so a second one may not go anywhere (nifu).
    assert_eq!(
        offered,
        ["KNIGHT", "LANCE", "ROOK"],
        "only the pieces that can be dropped somewhere are offered"
    );
    let mut refused = Session::resume(SHOGI, &s.state(), 0).unwrap();
    assert!(hand(&mut refused, "PAWN").is_err());
    // A knight may not be dropped where it could never move again, a lance not on the last rank.
    let knight = drops_of(&s, "KNIGHT");
    assert!(knight.contains(&"4,4".to_string()));
    assert!(!knight.contains(&"4,7".to_string()) && !knight.contains(&"4,8".to_string()));
    let lance = drops_of(&s, "LANCE");
    assert!(lance.contains(&"4,7".to_string()) && !lance.contains(&"4,8".to_string()));
    // Only empty squares, so a rook may go anywhere free.
    let rook = drops_of(&s, "ROOK");
    assert!(rook.contains(&"4,4".to_string()) && !rook.contains(&"4,2".to_string()));
}

#[test]
fn only_the_player_to_move_is_offered_drops() {
    let mut s = shogi_with_hand(&[("ROOK", 1)]);
    assert_eq!(s.view().choice.hand, vec!["ROOK".to_string()]);

    play(&mut s, (0, 2), (0, 3));

    let view = s.view();
    assert_eq!(view.turn, "GOTE");
    assert!(
        view.choice.hand.is_empty(),
        "GOTE holds nothing, SENTE is waiting"
    );
    assert_eq!(
        view.hand["SENTE"].get("ROOK"),
        Some(&1),
        "the hand is still shown"
    );
}

#[test]
fn dropping_places_the_piece_and_passes_the_turn() {
    let mut s = after_a_bishop_trade();

    drop_piece(&mut s, "BISHOP", "4,4");

    let view = s.view();
    assert!(view.pieces.iter().any(
        |p| (p.position.as_str(), p.code.as_str(), p.player.as_str()) == ("4,4", "BISHOP", "SENTE")
    ));
    assert!(view.hand["SENTE"].is_empty(), "the last one left the hand");
    assert_eq!(view.turn, "GOTE");
}

#[test]
fn dropping_twice_uses_the_hand_up() {
    let mut s = shogi_with_hand(&[("ROOK", 2)]);

    drop_piece(&mut s, "ROOK", "4,4");
    assert_eq!(s.view().hand["SENTE"].get("ROOK"), Some(&1));
    play(&mut s, (0, 6), (0, 5));
    drop_piece(&mut s, "ROOK", "3,4");

    assert!(s.view().hand["SENTE"].is_empty());
}

#[test]
fn a_click_on_an_illegal_square_only_puts_the_hand_piece_back() {
    let mut s = shogi_with_hand(&[("KNIGHT", 1)]);
    hand(&mut s, "KNIGHT").unwrap();

    at(&mut s, "4,8").unwrap(); // no knight on the last rank

    let view = s.view();
    assert!(
        view.choice.chosen.is_empty(),
        "the knight is back in the hand"
    );
    assert_eq!(view.turn, "SENTE");
    assert_eq!(view.hand["SENTE"].get("KNIGHT"), Some(&1));
}

#[test]
fn an_answer_off_the_board_changes_nothing() {
    let mut s = shogi_with_hand(&[("KNIGHT", 1)]);
    let before = json(&s);

    assert!(at(&mut s, "40,40").is_err(), "off the board");
    assert!(at(&mut s, "4,8").is_err(), "nothing is chosen to put back");

    assert_eq!(before, json(&s));
}

#[test]
fn a_piece_that_is_not_in_the_hand_cannot_be_chosen() {
    let mut s = shogi_with_hand(&[("ROOK", 1)]);
    let before = json(&s);

    assert!(hand(&mut s, "BISHOP").is_err());
    assert!(hand(&mut s, "NOT_A_PIECE").is_err());

    assert_eq!(before, json(&s));
}

#[test]
fn a_game_without_a_hand_refuses_the_hand() {
    let mut s = Session::new(CHESS).unwrap();
    let before = json(&s);

    assert!(hand(&mut s, "PAWN").is_err());

    assert_eq!(before, json(&s));
}

#[test]
fn a_pending_promotion_must_be_answered_before_choosing_from_the_hand() {
    let mut s = Session::new(SHOGI).unwrap();
    play(&mut s, (6, 2), (6, 3));
    play(&mut s, (2, 6), (2, 5));
    support::play(&mut s, "7,1", "1,7");
    let before = json(&s);
    assert!(
        !s.view().choice.codes.is_empty(),
        "the capture asks about promotion"
    );

    assert!(hand(&mut s, "BISHOP").is_err());
    assert!(
        s.view().choice.hand.is_empty(),
        "no hand piece is offered while the question is open"
    );

    assert_eq!(before, json(&s));
}

#[test]
fn a_selected_piece_is_dropped_from_the_hand_without_trouble() {
    let mut s = shogi_with_hand(&[("ROOK", 1)]);
    at(&mut s, "4,2").unwrap();
    assert_eq!(
        s.view().choice.start.as_ref().map(|p| p.as_str()),
        Some("4,2")
    );

    hand(&mut s, "ROOK").unwrap();
    assert_eq!(
        s.view().choice.start,
        None,
        "the picked-up piece is put back"
    );
    at(&mut s, "4,4").unwrap();

    let view = s.view();
    assert_eq!(view.turn, "GOTE");
}

#[test]
fn a_dropped_game_can_be_saved_and_resumed() {
    let mut s = after_a_bishop_trade();
    drop_piece(&mut s, "BISHOP", "4,4");

    let resumed = Session::resume(SHOGI, &s.state(), 7).unwrap();

    assert_eq!(json(&s), json(&resumed));
}
