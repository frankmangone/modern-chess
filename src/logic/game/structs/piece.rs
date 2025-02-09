use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Piece {
    pub code: String,
    pub player: String,
    pub total_moves: u16,
    pub state: HashMap<String, PieceState>,
}

#[derive(Clone, Debug)]
pub enum PieceState {
    Blank,
    Uint(u16),
    String(String)
}

impl Piece {
    pub fn new(code: String, player: String) -> Self {
        Piece {
            code,
            player,
            total_moves: 0u16,
            state: HashMap::new(),
        }
    }
}
