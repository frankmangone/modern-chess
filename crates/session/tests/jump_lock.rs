//! Mid-jump, the piece that is jumping stays the only one that can move, even if its player
//! clicks it again.
mod support;

use modern_chess_session::Session;

const JUMP: &str = include_str!("fixtures/jump.json");

#[test]
fn reselecting_the_jumping_piece_keeps_the_lock() {
    let mut s = Session::new(JUMP).unwrap();
    support::play(&mut s, "0,0", "0,2");

    support::at(&mut s, "0,2").unwrap();

    let view = s.view();
    assert_eq!(view.choice.start.as_ref().map(|p| p.as_str()), Some("0,2"));
    assert_eq!(support::destinations(&view), vec!["0,4".to_string()]);
    let err = support::at(&mut s, "2,0").unwrap_err();
    assert!(
        err.contains("jump"),
        "the lock survived the second click: {err}"
    );
}
