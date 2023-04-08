mod board;

pub use crate::board::{ Board, Position };

fn main() {
    let mut board = Board::new(8, 8);

    // Set up a chess board
    board.add_piece(&Position(0,0), "ROOK").ok();
    board.add_piece(&Position(0,1), "PAWN").ok();
    board.add_piece(&Position(0,6), "PAWN").ok();
    board.add_piece(&Position(0,6), "ROOK").ok();

    board.add_piece(&Position(1,0), "KNIGHT").ok();
    board.add_piece(&Position(1,1), "PAWN").ok();
    board.add_piece(&Position(1,6), "PAWN").ok();
    board.add_piece(&Position(1,6), "KNIGHT").ok();

    board.add_piece(&Position(2,0), "BISHOP").ok();
    board.add_piece(&Position(2,1), "PAWN").ok();
    board.add_piece(&Position(2,6), "PAWN").ok();
    board.add_piece(&Position(2,6), "BISHOP").ok();

    board.add_piece(&Position(3,0), "QUEEN").ok();
    board.add_piece(&Position(3,1), "PAWN").ok();
    board.add_piece(&Position(3,6), "PAWN").ok();
    board.add_piece(&Position(3,6), "QUEEN").ok();

    board.add_piece(&Position(4,0), "KING").ok();
    board.add_piece(&Position(4,1), "PAWN").ok();
    board.add_piece(&Position(4,6), "PAWN").ok();
    board.add_piece(&Position(4,6), "KING").ok();

    board.add_piece(&Position(5,0), "KNIGHT").ok();
    board.add_piece(&Position(5,1), "PAWN").ok();
    board.add_piece(&Position(5,6), "PAWN").ok();
    board.add_piece(&Position(5,6), "KNIGHT").ok();

    board.add_piece(&Position(6,0), "BISHOP").ok();
    board.add_piece(&Position(6,1), "PAWN").ok();
    board.add_piece(&Position(6,6), "PAWN").ok();
    board.add_piece(&Position(6,6), "BISHOP").ok();

    board.add_piece(&Position(7,0), "ROOK").ok();
    board.add_piece(&Position(7,1), "PAWN").ok();
    board.add_piece(&Position(7,6), "PAWN").ok();
    board.add_piece(&Position(7,6), "ROOK").ok();

    println!("This is how a chess board would look: {:?}", board.pieces);
}
