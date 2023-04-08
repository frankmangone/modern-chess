use std::collections::HashMap;

struct Board {
    pieces: HashMap<String, String>
}

enum BoardError {
    TileNotEmpty,
    OutOfBounds
}

impl Board {
    // Creates a new board struct
    fn new() -> Board {
        Board {
            pieces: HashMap::new()
        }
    }

    // Adds a piece to an existing board
    // TODO: Consider using some sort of encoding for the position
    fn add_piece(&mut self, position: &str, piece: &str) -> Result<(), BoardError> {        
        // Existing cannot place a piece in place of another (revisit this).
        if self.pieces.contains_key(position) {
            return Err(BoardError::TileNotEmpty);
        }

        // TODO: Out of bounds!

        self.pieces.insert(String::from(position), String::from(piece));
        Ok(())
    }
}

fn main() {
    let mut board = Board::new();
    println!("A board was created! It still has no pieces inside.");

    board.add_piece("2-1", "PAWN").ok();
    board.add_piece("1-1", "PAWN").ok();
    println!("A couple pawns were added! (2-1, 1-1)");
    
    println!("If we want to add a piece in an existing place, that should return an error and not alter state.");
    board.add_piece("1-1", "PAWN").ok();

    println!("We should now see only two pawns in the board instance: {:?}", board.pieces);
}
