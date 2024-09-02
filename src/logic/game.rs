use crate::specs::GameSpec;

use crate::logic::Board;

#[derive(Debug)]
pub struct Game {
    pub name: String,
    pub players: Vec<String>,
    pub board: Board
}

impl Game {
    pub fn from_spec(spec: GameSpec) -> Game {
        let players: Vec<String> = spec.players.into_iter().map(|x| x.name).collect();

        Game {
            name: spec.name,
            players,
            board: Board::from_spec(spec.board),
        }
    }
}