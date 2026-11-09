//! What each side has lost, and how the material stands, as the view reports them.

use modern_chess_session::session::Session;
use std::collections::BTreeMap;

const CHESS: &str = include_str!("../../engine/specs/chess.json");
const SHOGI: &str = include_str!("../../engine/specs/shogi.json");
const ATOMIC: &str = include_str!("../../engine/specs/atomic.json");
const MARCH: &str = include_str!("../../../apps/docs/examples/march.json");

fn play(session: &mut Session, from: (u8, u8), to: (u8, u8)) {
    session.select(from.0, from.1).unwrap();
    session.make_move(to.0, to.1).unwrap();
}

fn lost(session: &Session, player: &str) -> BTreeMap<String, u32> {
    session.view().captured[player].clone()
}

fn counts(pairs: &[(&str, u32)]) -> BTreeMap<String, u32> {
    pairs
        .iter()
        .map(|(code, n)| (code.to_string(), *n))
        .collect()
}

/// 1. e4 d5 2. exd5
fn white_takes_a_pawn(spec: &str) -> Session {
    let mut s = Session::new(spec).unwrap();
    play(&mut s, (4, 1), (4, 3));
    play(&mut s, (3, 6), (3, 4));
    play(&mut s, (4, 3), (3, 4));
    s
}

#[test]
fn a_new_game_has_an_empty_entry_for_every_player() {
    let view = Session::new(CHESS).unwrap().view();

    assert_eq!(view.captured.len(), 2);
    assert!(view.captured["WHITE"].is_empty());
    assert!(view.captured["BLACK"].is_empty());
}

#[test]
fn a_captured_piece_is_listed_under_the_side_that_lost_it() {
    let s = white_takes_a_pawn(CHESS);

    assert_eq!(lost(&s, "BLACK"), counts(&[("PAWN", 1)]));
    assert!(lost(&s, "WHITE").is_empty());
}

#[test]
fn castling_moves_a_rook_without_losing_one() {
    let mut s = Session::new(CHESS).unwrap();
    play(&mut s, (4, 1), (4, 3));
    play(&mut s, (4, 6), (4, 4));
    play(&mut s, (6, 0), (5, 2));
    play(&mut s, (6, 7), (5, 5));
    play(&mut s, (5, 0), (4, 1));
    play(&mut s, (5, 7), (4, 6));
    play(&mut s, (4, 0), (6, 0));

    assert!(lost(&s, "WHITE").is_empty());
    assert!(lost(&s, "BLACK").is_empty());
}

#[test]
fn an_explosion_counts_the_capturing_piece_too() {
    let s = white_takes_a_pawn(ATOMIC);

    assert_eq!(lost(&s, "WHITE"), counts(&[("PAWN", 1)]));
    assert_eq!(lost(&s, "BLACK"), counts(&[("PAWN", 1)]));
}

#[test]
fn losses_survive_saving_and_resuming() {
    let saved = white_takes_a_pawn(CHESS).state();

    let resumed = Session::resume(CHESS, &saved, 3).unwrap();

    assert_eq!(lost(&resumed, "BLACK"), counts(&[("PAWN", 1)]));
}

#[test]
fn a_game_saved_before_losses_were_kept_resumes_with_none() {
    let mut saved = white_takes_a_pawn(CHESS).state();
    saved.as_object_mut().unwrap().remove("captured");

    let resumed = Session::resume(CHESS, &saved, 3).unwrap();

    assert!(lost(&resumed, "BLACK").is_empty());
}

#[test]
fn material_adds_up_the_pieces_still_on_the_board() {
    let start = Session::new(CHESS).unwrap().view();
    assert_eq!(start.material["WHITE"], 39);
    assert_eq!(start.material["BLACK"], 39);

    let after = white_takes_a_pawn(CHESS).view();
    assert_eq!(after.material["WHITE"], 39);
    assert_eq!(after.material["BLACK"], 38);
}

#[test]
fn a_game_whose_pieces_have_no_value_reports_no_material() {
    assert!(Session::new(MARCH).unwrap().view().material.is_empty());
}

#[test]
fn a_promotion_loses_nothing_and_a_held_piece_still_counts_as_material() {
    let mut s = Session::new(SHOGI).unwrap();
    play(&mut s, (6, 2), (6, 3));
    play(&mut s, (2, 6), (2, 5));
    s.select(7, 1).unwrap();
    s.make_move(1, 7).unwrap();
    s.promote("HORSE").unwrap();

    assert!(lost(&s, "SENTE").is_empty());
    assert_eq!(lost(&s, "GOTE"), counts(&[("BISHOP", 1)]));
    // SENTE: 61 to start, the bishop became a horse (+2) and one is in hand (+8).
    assert_eq!(s.view().material["SENTE"], 71);
    assert_eq!(s.view().material["GOTE"], 53);
}
