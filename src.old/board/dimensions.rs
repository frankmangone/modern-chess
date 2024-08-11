pub struct Dimensions(pub u8, pub u8);

impl Dimensions {
  /// Creates a position from row/column data
  pub fn new(row: u8, column: u8) -> Self {
      Self(row, column)
  }

  /// Gets the row of a position
  pub fn rows(&self) -> u8 {
      self.0
  }

  /// Gets the column of a position
  pub fn cols(&self) -> u8 {
      self.1
  }
}