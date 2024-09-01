mod specs;
mod structs;

use specs::GameSpecError;

use crate::specs::parse_game_spec;
use crate::structs::Game;

fn main() -> Result<(), GameSpecError> {
    let game = parse_game_spec("specs/games/chess.json")?;
    
    println!("Pieces: {:?}", game.pieces);

    let game = Game::from_spec(game);
    
    println!("Parsed game: {:?}", game);
    println!("Name: {:?}", game.name);
    println!("Players: {:?}", game.players);

    Ok(())
}
