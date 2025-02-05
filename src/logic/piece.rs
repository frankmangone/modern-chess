#[derive(Clone, Debug)]
pub struct Piece {
    pub code: String,
    pub player: String,
}

impl Piece {
    pub fn new(code: String, player: String) -> Self {
        Piece {
            code,
            player,
        }
    }
}
