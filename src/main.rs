mod board;

pub use crate::board::{ Board, Position };

fn main() {
    let mut board = Board::new(8, 8);
    println!("A board was created! It still has no pieces inside.");

    let pos1 = Position(1,1);
    let pos2 = Position(2,1);
    let pos3 = Position(8,0);

    board.add_piece(&pos1, "PAWN").ok();
    board.add_piece(&pos2, "PAWN").ok();
    println!("A couple pawns were added! (2-1, 1-1)");
    
    println!("If we want to add a piece in an existing place, that should return an error and not alter state.");
    board.add_piece(&pos2, "PAWN").ok();

    println!("Also, adding a piece out of bounds doesn't work.");
    board.add_piece(&pos3, "PAWN").ok();

    println!("We should now see only two pawns in the board instance: {:?}", board.pieces);
}
