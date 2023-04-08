use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Position(u8, u8); // Positions are zero-indexed
struct Dimensions(u8, u8);

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

struct Board {
    pieces: HashMap<Position, String>,
    dimensions: Dimensions,
}

enum BoardError {
    TileNotEmpty,
    OutOfBounds
}

impl Board {
    //
    // Creates a new board struct
    fn new(rows: u8, cols: u8) -> Board {
        Board {
            pieces: HashMap::new(),
            dimensions: Dimensions(rows, cols),
        }
    }

    //
    // Adds a piece to an existing board
    // TODO: Consider using some sort of encoding for the position
    fn add_piece(&mut self, position: &Position, piece: &str) -> Result<(), BoardError> {        
        // Existing cannot place a piece in place of another (revisit this).
        if self.pieces.contains_key(position) {
            return Err(BoardError::TileNotEmpty);
        }

        // Check if position is out of bounds
        if self.dimensions.0 <= position.0 || self.dimensions.1 <= position.1 {
            return Err(BoardError::OutOfBounds);
        }

        self.pieces.insert(*position, String::from(piece));
        Ok(())
    }
}

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
