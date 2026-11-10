//! Games played by placing stones from a supply, passing and counting the board, as in Go.

use modern_chess_session::session::Session;
use modern_chess_session::Status;

const GO: &str = include_str!("../../engine/specs/go.json");
const CHESS: &str = include_str!("../../engine/specs/chess.json");

fn go() -> Session {
    Session::new(GO).unwrap()
}

fn place(session: &mut Session, x: u8, y: u8) {
    session.drop_piece("STONE", x, y).unwrap();
}

#[test]
fn a_supply_game_offers_every_empty_point_without_a_hand() {
    let view = go().view();

    assert!(!view.hands, "nothing is captured into a hand");
    assert!(view.hand.is_empty());
    assert_eq!(view.supply, vec!["STONE".to_string()]);
    assert_eq!(view.drops.get("STONE").map(Vec::len), Some(81));
}

#[test]
fn a_stone_can_be_placed_straight_from_the_supply() {
    let mut s = go();

    place(&mut s, 4, 4);

    let view = s.view();
    assert_eq!(view.turn, "WHITE");
    assert_eq!(view.pieces.len(), 1);
    assert_eq!(view.drops.get("STONE").map(Vec::len), Some(80));
}

#[test]
fn a_stone_cannot_go_on_an_occupied_point() {
    let mut s = go();
    place(&mut s, 4, 4);

    let error = s.drop_piece("STONE", 4, 4).unwrap_err();

    assert_eq!(error, "not a legal square for that piece");
    assert_eq!(s.view().turn, "WHITE", "nothing changed");
}

#[test]
fn a_piece_that_is_not_in_the_supply_is_refused() {
    let mut s = go();

    let error = s.drop_piece("KING", 4, 4).unwrap_err();

    assert_eq!(error, "that piece is not in your hand");
}

#[test]
fn captured_stones_show_up_in_the_view() {
    let mut s = go();
    // WHITE's stone at (1,0) is taken by BLACK at (0,0), (2,0) and (1,1).
    place(&mut s, 0, 0);
    place(&mut s, 1, 0);
    place(&mut s, 2, 0);
    place(&mut s, 8, 8);
    place(&mut s, 1, 1);

    let view = s.view();

    assert_eq!(view.captured["WHITE"].get("STONE"), Some(&1));
    assert!(view.pieces.iter().all(|p| !(p.x == 1 && p.y == 0)));
}

#[test]
fn a_point_with_no_liberties_is_not_offered() {
    let mut s = go();
    // BLACK 1,0 / WHITE elsewhere / BLACK 0,1: the corner is now a point WHITE cannot use.
    place(&mut s, 1, 0);
    place(&mut s, 8, 8);
    place(&mut s, 0, 1);

    let view = s.view();

    assert_eq!(view.turn, "WHITE");
    assert!(!view.drops["STONE"].contains(&[0, 0]));
}

#[test]
fn passing_gives_the_turn_over_and_counts_the_passes() {
    let mut s = go();
    assert!(s.view().can_pass);
    assert_eq!(s.view().passes, 0);

    s.pass().unwrap();

    let view = s.view();
    assert_eq!(view.turn, "WHITE");
    assert_eq!(view.passes, 1);
}

#[test]
fn a_move_in_between_resets_the_count_of_passes() {
    let mut s = go();
    s.pass().unwrap();
    place(&mut s, 4, 4);

    assert_eq!(s.view().passes, 0);
}

#[test]
fn two_passes_in_a_row_end_the_game_and_name_the_winner() {
    let mut s = go();
    place(&mut s, 0, 0);
    place(&mut s, 8, 8);
    s.pass().unwrap();
    s.pass().unwrap();

    // One stone each, a single empty region that touches both, and 7.5 for WHITE.
    let view = s.view();
    assert_eq!(
        view.status,
        Status::Won {
            winner: "WHITE".into()
        }
    );
    assert!(!view.can_pass);
    assert!(view.drops.is_empty(), "nothing can be played any more");
}

#[test]
fn the_view_shows_the_score_as_if_the_game_ended_now() {
    let mut s = go();
    assert_eq!(view_score(&s, "BLACK"), 0.0);
    assert_eq!(view_score(&s, "WHITE"), 7.5);

    place(&mut s, 4, 4);

    // The one empty region touches only BLACK's stone, so by area scoring it is all theirs.
    assert_eq!(view_score(&s, "BLACK"), 81.0);
    assert_eq!(view_score(&s, "WHITE"), 7.5);
}

fn view_score(session: &Session, player: &str) -> f64 {
    session.view().score[player]
}

#[test]
fn nothing_can_be_played_after_the_game_is_over() {
    let mut s = go();
    s.pass().unwrap();
    s.pass().unwrap();

    assert_eq!(s.pass().unwrap_err(), "the game is over");
    assert_eq!(s.drop_piece("STONE", 0, 0).unwrap_err(), "the game is over");
}

#[test]
fn a_game_that_does_not_allow_passing_says_so() {
    let mut s = Session::new(CHESS).unwrap();

    assert_eq!(s.pass().unwrap_err(), "passing is not allowed in this game");
    let view = s.view();
    assert!(!view.can_pass);
    assert!(view.supply.is_empty());
    assert!(view.score.is_empty());
    assert_eq!(view.passes, 0);
}

#[test]
fn the_game_picks_up_where_it_was_left() {
    let mut s = go();
    place(&mut s, 4, 4);
    s.pass().unwrap();

    let resumed = Session::resume(GO, &s.state(), 2).unwrap();

    assert_eq!(resumed.view(), s.view());
    assert_eq!(resumed.view().passes, 1);
}

#[test]
fn a_view_of_a_supply_game_carries_the_new_fields_in_its_json() {
    let json = serde_json::to_value(go().view()).unwrap();

    assert_eq!(json["supply"], serde_json::json!(["STONE"]));
    assert_eq!(json["can_pass"], serde_json::json!(true));
    assert_eq!(json["passes"], serde_json::json!(0));
    assert_eq!(json["score"]["WHITE"], serde_json::json!(7.5));
}
