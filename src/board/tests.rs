#[cfg(test)]
mod tests {
  use crate::board::{ BoardError, Board, Position };

  #[test]
  fn adding_piece_to_valid_tile_works() {
    let mut board = Board::new(8, 8);

    let pos = Position(1,1);
    board.add_piece(&pos, "PAWN").ok();

    assert_eq!(board.pieces.get(&pos).unwrap(), "PAWN");
  }

  #[test]
  fn adding_piece_to_occupied_tile_fails() {
    let mut board = Board::new(8, 8);

    let pos = Position(1,1);
    board.add_piece(&pos, "PAWN").ok();
    
    let result = board.add_piece(&pos, "PAWN");

    match result {
      Ok(_) => assert!(false),
      Err(error) => {
        match error {
          BoardError::TileNotEmpty => assert!(true),
          _ => assert!(false)
        }
      }
    }
  }

  #[test]
  fn adding_piece_out_of_bounds_fails() {
    let mut board = Board::new(8, 8);

    let pos = Position(8,1);
    let result = board.add_piece(&pos, "PAWN");

    match result {
      Ok(_) => assert!(false),
      Err(error) => {
        match error {
          BoardError::OutOfBounds => assert!(true),
          _ => assert!(false)
        }
      }
    }
  }
}
