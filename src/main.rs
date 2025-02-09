mod logic;
mod shared;
mod specs;
mod tests;

use specs::GameSpecError;

use crate::specs::parse_game_spec;
use crate::logic::Game;

fn main() -> Result<(), GameSpecError> {
    let game_spec = parse_game_spec("specs/chess.json")?;
    let mut game = Game::from_spec(game_spec);
    
    // println!("Parsed game: {:?}", game);

    // TEMP: Test move calculation
    // game.calculate_moves(vec![6, 0]);
    // println!("Knight @[6, 0] moves: {:?}", game.available_moves);

    // game.calculate_moves(vec![5, 0]);
    // println!("Bishop @[5, 0] moves: {:?}", game.available_moves);

    game.calculate_moves(vec![4, 1]);
    println!("Pawn @[4, 1] moves: {:?}", game.available_moves);

    Ok(())
}
