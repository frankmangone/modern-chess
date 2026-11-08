//! Mid-jump, the piece that is jumping stays the only one that can move, even if its player
//! clicks it again.
use modern_chess_session::Session;

const JUMP: &str = include_str!("fixtures/jump.json");

#[test]
fn reselecting_the_jumping_piece_keeps_the_lock() {
    let mut s = Session::new(JUMP).unwrap();
    s.select(0, 0).unwrap();
    s.make_move(0, 2).unwrap();

    s.select(0, 2).unwrap();

    assert_eq!(s.view().selected, Some([0, 2]));
    assert_eq!(s.view().destinations, vec![[0, 4]]);
    let err = s.select(2, 0).unwrap_err();
    assert!(
        err.contains("jump"),
        "the lock survived the second click: {err}"
    );
}
