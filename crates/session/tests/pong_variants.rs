//! The variants of Pong hau k'i: Do-guti, which places its stones before sliding them, and two
//! layouts of our own. The lines played here were found by solving each game from every position
//! (a position is won, lost or drawn with best play), so they pin what the specs are meant to do.

mod support;

use modern_chess::logic::ChoiceKind;
use modern_chess_session::{Session, Status};

const DO_GUTI: &str = include_str!("../../engine/specs/do_guti.json");
const SPUR: &str = include_str!("../../engine/specs/pong_hau_ki_spur.json");
const FAN: &str = include_str!("../../engine/specs/pong_hau_ki_fan.json");

fn play_all(session: &mut Session, moves: &[(&str, &str)]) {
    for (from, to) in moves {
        support::play(session, from, to);
    }
}

fn place_all(session: &mut Session, points: &[&str]) {
    for point in points {
        support::at(session, point).unwrap();
    }
}

fn winner(session: &Session) -> Option<String> {
    match session.view().status {
        Status::Won { winner } => Some(winner),
        _ => None,
    }
}

#[test]
fn do_guti_begins_on_an_empty_board_and_places_one_stone_a_turn() {
    let mut game = Session::new(DO_GUTI).unwrap();
    let view = game.view();
    assert!(view.pieces.is_empty(), "no stone is down at the start");
    assert_eq!(support::names_for(&view, ChoiceKind::Place).len(), 5);

    support::at(&mut game, "centre").unwrap();
    let view = game.view();
    assert_eq!(view.turn, "BLACK");
    assert_eq!(view.pieces.len(), 1);
    assert_eq!(
        support::names_for(&view, ChoiceKind::Place).len(),
        4,
        "the occupied point is not offered"
    );
    assert!(
        support::at(&mut game, "centre").is_err(),
        "a stone cannot be placed on a stone"
    );
}

#[test]
fn do_guti_slides_once_all_four_stones_are_down() {
    let mut game = Session::new(DO_GUTI).unwrap();
    // White: top-left and centre. Black: top-right and bottom-right. Only bottom-left is empty.
    place_all(
        &mut game,
        &["top-left", "top-right", "centre", "bottom-right"],
    );
    let view = game.view();
    assert_eq!(view.turn, "WHITE");
    assert_eq!(view.pieces.len(), 4);
    assert_eq!(winner(&game), None);

    // Placing is over: a stone is chosen to slide, and the only place to slide to is bottom-left.
    support::at(&mut game, "top-left").unwrap();
    assert_eq!(support::destinations(&game.view()), ["bottom-left"]);
    support::at(&mut game, "bottom-left").unwrap();
    // Black's stones are at top-right and bottom-right, shut in by the centre and bottom-left.
    assert_eq!(winner(&game).as_deref(), Some("WHITE"));
}

#[test]
fn do_guti_can_be_lost_by_placing() {
    let mut game = Session::new(DO_GUTI).unwrap();
    // White's stones on top-left and bottom-left are shut in by Black's on centre and bottom-right.
    place_all(
        &mut game,
        &["top-left", "centre", "bottom-left", "bottom-right"],
    );
    assert_eq!(winner(&game).as_deref(), Some("BLACK"));
}

#[test]
fn the_spur_opens_with_a_single_move() {
    let mut game = Session::new(SPUR).unwrap();
    let view = game.view();
    assert_eq!(view.pieces.len(), 4);
    // The stone on the bottom right is shut in; the one on the left can only go to the top.
    support::at(&mut game, "left").unwrap();
    assert_eq!(support::destinations(&game.view()), ["top"]);
}

#[test]
fn the_spur_can_be_lost_by_a_careless_slide() {
    let mut game = Session::new(SPUR).unwrap();
    play_all(
        &mut game,
        &[
            ("left", "top"),
            ("bottom-left", "left"),
            ("top", "spur"),
            ("left", "top"),
            ("bottom-right", "bottom-left"),
            ("right", "bottom-right"),
            ("bottom-left", "left"),
        ],
    );
    assert_eq!(winner(&game), None);
    support::play(&mut game, "bottom-right", "bottom-left");
    assert_eq!(
        winner(&game).as_deref(),
        Some("BLACK"),
        "White's stones are shut in"
    );
}

#[test]
fn the_fan_is_lost_in_three_plies_by_the_wrong_answer() {
    let mut game = Session::new(FAN).unwrap();
    play_all(
        &mut game,
        &[("left", "hub"), ("right", "left"), ("far-right", "right")],
    );
    assert_eq!(
        winner(&game).as_deref(),
        Some("WHITE"),
        "Black's stones are shut in"
    );

    // The other answer to the same opening is safe.
    let mut game = Session::new(FAN).unwrap();
    play_all(&mut game, &[("left", "hub"), ("far-left", "left")]);
    assert_eq!(winner(&game), None);
}
