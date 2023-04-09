#[cfg(test)]
mod tests {
  use crate::board::{ BoardError, Board, Position };
  use crate::piece::{ Piece };

  #[test]
  fn adding_piece_to_valid_tile_works() {
    let mut board = Board::new(8, 8);
    let piece = Piece::new(String::from("pawn"), 0, vec![]);
    let pos = Position(1,1);

    board.add_piece(&pos, &piece).ok();

    assert_eq!(board.pieces.get(&pos).unwrap(), &piece);
  }

  #[test]
  fn adding_piece_to_occupied_tile_fails() {
    let mut board = Board::new(8, 8);
    let piece = Piece::new(String::from("pawn"), 0, vec![]);
    let pos = Position(1,1);

    board.add_piece(&pos, &piece).ok();
    
    let result = board.add_piece(&pos, &piece);

    match result {
      Ok(_) => assert!(false),
      Err(error) => {
        match error {
          BoardError::PositionNotEmpty => assert!(true),
          _ => assert!(false)
        }
      }
    }
  }

  #[test]
  fn adding_piece_out_of_bounds_fails() {
    let mut board = Board::new(8, 8);
    let piece = Piece::new(String::from("pawn"), 0, vec![]);
    let pos = Position(8,1);

    let result = board.add_piece(&pos, &piece);

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
