mod piece;
mod board;

pub use crate::board::{ Board, Position };
pub use crate::piece::{ Piece };

fn main() {
    let mut board = Board::new(8, 8);

    let white_pawn = Piece::new("pawn", 0);
    let white_rook = Piece::new("rook", 0);
    let white_knight = Piece::new("knight", 0);
    let white_bishop = Piece::new("bishop", 0);
    let white_queen = Piece::new("queen", 0);
    let white_king = Piece::new("king", 0);

    let black_pawn = Piece::new("pawn", 1);
    let black_rook = Piece::new("rook", 1);
    let black_knight = Piece::new("knight", 1);
    let black_bishop = Piece::new("bishop", 1);
    let black_queen = Piece::new("queen", 1);
    let black_king = Piece::new("king", 1);

    // Set up a chess board
    board.add_piece(&Position(0,0), &white_rook).ok();
    board.add_piece(&Position(0,1), &white_pawn).ok();
    board.add_piece(&Position(0,6), &black_pawn).ok();
    board.add_piece(&Position(0,6), &black_rook).ok();

    board.add_piece(&Position(1,0), &white_knight).ok();
    board.add_piece(&Position(1,1), &white_pawn).ok();
    board.add_piece(&Position(1,6), &black_pawn).ok();
    board.add_piece(&Position(1,6), &black_knight).ok();

    board.add_piece(&Position(2,0), &white_bishop).ok();
    board.add_piece(&Position(2,1), &white_pawn).ok();
    board.add_piece(&Position(2,6), &black_pawn).ok();
    board.add_piece(&Position(2,6), &black_bishop).ok();

    board.add_piece(&Position(3,0), &white_queen).ok();
    board.add_piece(&Position(3,1), &white_pawn).ok();
    board.add_piece(&Position(3,6), &black_pawn).ok();
    board.add_piece(&Position(3,6), &black_queen).ok();

    board.add_piece(&Position(4,0), &white_king).ok();
    board.add_piece(&Position(4,1), &white_pawn).ok();
    board.add_piece(&Position(4,6), &black_pawn).ok();
    board.add_piece(&Position(4,6), &black_king).ok();

    board.add_piece(&Position(5,0), &white_bishop).ok();
    board.add_piece(&Position(5,1), &white_pawn).ok();
    board.add_piece(&Position(5,6), &black_pawn).ok();
    board.add_piece(&Position(5,6), &black_bishop).ok();

    board.add_piece(&Position(6,0), &white_knight).ok();
    board.add_piece(&Position(6,1), &white_pawn).ok();
    board.add_piece(&Position(6,6), &black_pawn).ok();
    board.add_piece(&Position(6,6), &black_knight).ok();

    board.add_piece(&Position(7,0), &white_rook).ok();
    board.add_piece(&Position(7,1), &white_pawn).ok();
    board.add_piece(&Position(7,6), &black_pawn).ok();
    board.add_piece(&Position(7,6), &black_rook).ok();

    println!("This is how a chess board would look: {:?}", board.pieces);
}
