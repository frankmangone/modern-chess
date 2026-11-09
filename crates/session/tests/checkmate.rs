//! The end of a game: a checkmate must leave a session that can still be looked at.

use modern_chess_session::session::Session;
use modern_chess_session::view::Status;

const CHESS: &str = include_str!("../../engine/specs/chess.json");

fn play(session: &mut Session, from: (u8, u8), to: (u8, u8)) {
    session.select(from.0, from.1).unwrap();
    session.make_move(to.0, to.1).unwrap();
}

/// The first player is mated here, and the second player mates in `scholars_mate`.
fn fools_mate() -> Session {
    let mut s = Session::new(CHESS).unwrap();
    play(&mut s, (5, 1), (5, 2));
    play(&mut s, (4, 6), (4, 4));
    play(&mut s, (6, 1), (6, 3));
    play(&mut s, (3, 7), (7, 3));
    s
}

fn scholars_mate() -> Session {
    let mut s = Session::new(CHESS).unwrap();
    play(&mut s, (4, 1), (4, 3));
    play(&mut s, (4, 6), (4, 4));
    play(&mut s, (3, 0), (7, 4));
    play(&mut s, (1, 7), (2, 5));
    play(&mut s, (5, 0), (2, 3));
    play(&mut s, (6, 7), (5, 5));
    play(&mut s, (7, 4), (5, 6));
    s
}

#[test]
fn a_checkmated_game_can_still_be_viewed_and_names_the_winner() {
    let view = fools_mate().view();

    assert_eq!(view.status, Status::Won { winner: "BLACK".to_string() });
}

#[test]
fn a_checkmated_game_survives_being_saved_and_resumed() {
    let saved = fools_mate().state();

    let view = Session::resume(CHESS, &saved, 4).unwrap().view();

    assert_eq!(view.status, Status::Won { winner: "BLACK".to_string() });
}

#[test]
fn a_game_won_by_the_first_player_can_still_be_viewed() {
    let view = scholars_mate().view();

    assert_eq!(view.status, Status::Won { winner: "WHITE".to_string() });
}

#[test]
fn a_game_won_by_the_first_player_survives_being_saved_and_resumed() {
    let saved = scholars_mate().state();

    let view = Session::resume(CHESS, &saved, 7).unwrap().view();

    assert_eq!(view.status, Status::Won { winner: "WHITE".to_string() });
}
