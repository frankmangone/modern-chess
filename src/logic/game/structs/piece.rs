#[derive(Clone, Debug)]
pub struct Piece {
    pub code: String,
    pub player: String,
    pub total_moves: u16,
}

impl Piece {
    pub fn new(code: String, player: String) -> Self {
        Piece {
            code,
            player,
            total_moves: 0u16,
        }
    }
}
