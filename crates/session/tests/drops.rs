//! Playing pieces from the hand, as in Shogi: what the view offers, and what a drop does.

use modern_chess_session::session::Session;
use serde_json::json;

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

fn play(session: &mut Session, from: (u8, u8), to: (u8, u8)) {
    session.select(from.0, from.1).unwrap();
    session.make_move(to.0, to.1).unwrap();
}

/// Both sides open the bishops' diagonal and SENTE takes GOTE's bishop, so SENTE holds one.
fn after_a_bishop_trade() -> Session {
    let mut s = Session::new(SHOGI).unwrap();
    play(&mut s, (6, 2), (6, 3)); // SENTE opens the diagonal
    play(&mut s, (2, 6), (2, 5)); // GOTE opens it too
    s.select(7, 1).unwrap();
    s.make_move(1, 7).unwrap(); // takes the bishop, landing in the promotion zone
    s.promote("HORSE").unwrap();
    play(&mut s, (0, 6), (0, 5)); // GOTE plays elsewhere
    s
}

#[test]
fn a_hand_game_can_be_played_on_the_board() {
    let mut s = Session::new(SHOGI).unwrap();

    s.select(4, 2).unwrap();
    s.make_move(4, 3).unwrap();

    assert_eq!(s.view().turn, "GOTE");
}

#[test]
fn the_view_shows_every_hand_and_starts_empty() {
    let view = Session::new(SHOGI).unwrap().view();

    assert!(view.hands);
    assert_eq!(view.hand.len(), 2, "one entry per player");
    assert!(view.hand.values().all(|hand| hand.is_empty()));
    assert!(view.drops.is_empty());
}

#[test]
fn a_game_without_a_hand_has_no_hand_in_its_view() {
    let view = Session::new(CHESS).unwrap().view();

    assert!(!view.hands);
    assert!(view.hand.is_empty());
    assert!(view.drops.is_empty());
}

#[test]
fn a_capture_puts_the_piece_in_the_capturers_hand() {
    let s = after_a_bishop_trade();

    let view = s.view();

    assert_eq!(view.hand["SENTE"].get("BISHOP"), Some(&1));
    assert!(view.hand["GOTE"].is_empty());
}

#[test]
fn the_view_lists_where_each_hand_piece_may_be_dropped() {
    let s = shogi_with_hand(&[("PAWN", 1), ("KNIGHT", 1), ("LANCE", 1), ("ROOK", 1)]);

    let drops = s.view().drops;

    assert_eq!(
        drops.keys().map(String::as_str).collect::<Vec<_>>(),
        ["KNIGHT", "LANCE", "PAWN", "ROOK"],
        "one entry for each kind in the hand"
    );
    // SENTE has a pawn on every file, so a second one may not go anywhere (nifu).
    assert!(drops["PAWN"].is_empty(), "{:?}", drops["PAWN"]);
    // A knight may not be dropped where it could never move again, a lance not on the last rank.
    assert!(drops["KNIGHT"].contains(&[4, 4]));
    assert!(!drops["KNIGHT"].contains(&[4, 7]) && !drops["KNIGHT"].contains(&[4, 8]));
    assert!(drops["LANCE"].contains(&[4, 7]) && !drops["LANCE"].contains(&[4, 8]));
    // Only empty squares, so a rook may go anywhere free.
    assert!(drops["ROOK"].contains(&[4, 4]) && !drops["ROOK"].contains(&[4, 2]));
}

#[test]
fn only_the_player_to_move_is_offered_drops() {
    let mut s = shogi_with_hand(&[("ROOK", 1)]);
    assert!(s.view().drops.contains_key("ROOK"));

    play(&mut s, (0, 2), (0, 3));

    let view = s.view();
    assert_eq!(view.turn, "GOTE");
    assert!(
        view.drops.is_empty(),
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

    s.drop_piece("BISHOP", 4, 4).unwrap();

    let view = s.view();
    assert!(view
        .pieces
        .iter()
        .any(|p| (p.x, p.y, p.code.as_str(), p.player.as_str()) == (4, 4, "BISHOP", "SENTE")));
    assert!(view.hand["SENTE"].is_empty(), "the last one left the hand");
    assert_eq!(view.turn, "GOTE");
}

#[test]
fn dropping_twice_uses_the_hand_up() {
    let mut s = shogi_with_hand(&[("ROOK", 2)]);

    s.drop_piece("ROOK", 4, 4).unwrap();
    assert_eq!(s.view().hand["SENTE"].get("ROOK"), Some(&1));
    play(&mut s, (0, 6), (0, 5));
    s.drop_piece("ROOK", 3, 4).unwrap();

    assert!(s.view().hand["SENTE"].is_empty());
}

#[test]
fn a_drop_on_an_illegal_square_changes_nothing() {
    let mut s = shogi_with_hand(&[("KNIGHT", 1)]);
    let before = json(&s);

    assert!(
        s.drop_piece("KNIGHT", 4, 8).is_err(),
        "no knight on the last rank"
    );
    assert!(s.drop_piece("KNIGHT", 4, 2).is_err(), "the square is taken");
    assert!(s.drop_piece("KNIGHT", 40, 40).is_err(), "off the board");

    assert_eq!(before, json(&s));
}

#[test]
fn a_piece_that_is_not_in_the_hand_cannot_be_dropped() {
    let mut s = shogi_with_hand(&[("ROOK", 1)]);
    let before = json(&s);

    assert!(s.drop_piece("BISHOP", 4, 4).is_err());
    assert!(s.drop_piece("NOT_A_PIECE", 4, 4).is_err());

    assert_eq!(before, json(&s));
}

#[test]
fn a_game_without_a_hand_refuses_drops() {
    let mut s = Session::new(CHESS).unwrap();
    let before = json(&s);

    assert!(s.drop_piece("PAWN", 4, 4).is_err());

    assert_eq!(before, json(&s));
}

#[test]
fn a_pending_promotion_must_be_answered_before_dropping() {
    let mut s = Session::new(SHOGI).unwrap();
    play(&mut s, (6, 2), (6, 3));
    play(&mut s, (2, 6), (2, 5));
    s.select(7, 1).unwrap();
    s.make_move(1, 7).unwrap();
    let before = json(&s);
    assert!(
        s.view().promotion.is_some(),
        "the capture asks about promotion"
    );

    assert!(s.drop_piece("BISHOP", 4, 4).is_err());
    assert!(
        s.view().drops.is_empty(),
        "no drops are offered while the question is open"
    );

    assert_eq!(before, json(&s));
}

#[test]
fn a_selected_piece_is_dropped_from_the_hand_without_trouble() {
    let mut s = shogi_with_hand(&[("ROOK", 1)]);
    s.select(4, 2).unwrap();
    assert_eq!(s.view().selected, Some([4, 2]));

    s.drop_piece("ROOK", 4, 4).unwrap();

    let view = s.view();
    assert_eq!(view.selected, None, "the picked-up piece is put back");
    assert_eq!(view.turn, "GOTE");
}

#[test]
fn a_dropped_game_can_be_saved_and_resumed() {
    let mut s = after_a_bishop_trade();
    s.drop_piece("BISHOP", 4, 4).unwrap();

    let resumed = Session::resume(SHOGI, &s.state(), 7).unwrap();

    assert_eq!(json(&s), json(&resumed));
}
