mod logic;
mod shared;
mod specs;

use specs::GameSpecError;

use crate::specs::parse_game_spec;
use crate::logic::Game;

fn main() -> Result<(), GameSpecError> {
    let game_spec = parse_game_spec("specs/games/chess.json")?;
    let game = Game::from_spec(game_spec);
    
    println!("Parsed game: {:?}", game);
    println!("Name: {:?}", game.name);
    println!("Players: {:?}", game.players);
    println!("Board: {:?}", game.board);

    Ok(())
}
