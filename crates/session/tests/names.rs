//! A name that is not a node of the board is refused, and the game is left as it was.

mod support;

use modern_chess_session::session::Session;

const CHESS: &str = include_str!("../../engine/specs/chess.json");

#[test]
fn an_unknown_name_is_refused_and_leaves_the_game_as_it_was() {
    let mut s = Session::new(CHESS).unwrap();
    let before = s.view();
    let refusal = Err("that square is not on the board".to_string());
    for name in ["", "4, 1", "e4", "4,1,0", "9,9"] {
        assert_eq!(support::at(&mut s, name), refusal);
        assert_eq!(s.view(), before);
    }
}
