//! The browser board and the game server must describe a position the same way.
use modern_chess_session::Session;
use modern_chess_wasm::Board;

const CHESS: &str = include_str!("../../engine/specs/chess.json");

#[test]
fn wasm_and_server_views_are_identical_for_the_same_position() {
    // The browser: a Board played move by move.
    let mut board = Board::new(CHESS).unwrap();
    board.select(4, 1).unwrap();
    board.make_move(4, 3).unwrap();
    board.select(4, 6).unwrap();

    // The server: a Session resumed from the state it stored after the same commands.
    let mut played = Session::new(CHESS).unwrap();
    played.select(4, 1).unwrap();
    played.make_move(4, 3).unwrap();
    played.select(4, 6).unwrap();
    let server = Session::resume(CHESS, &played.state(), 3).unwrap();

    assert_eq!(board.view(), serde_json::to_string(&server.view()).unwrap());
}
