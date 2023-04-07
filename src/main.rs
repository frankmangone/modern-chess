use std::collections::HashMap;

struct Board {
    pieces: HashMap<String, String>
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
    fn add_piece(&mut self, position: String, piece: String) {
        // TODO: Check if piece is already present!
        self.pieces.insert(position, piece);
    }
}

fn main() {
    let mut board = Board::new();
    println!("A board was created! It still has no pieces inside.");

    board.add_piece(String::from("1-1"), String::from("PAWN"));
    println!("A pawn was added!");

    println!("We should now see that in the board instance: {:?}", board.pieces);
}
