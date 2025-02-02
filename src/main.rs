mod logic;
mod shared;
mod specs;

use specs::GameSpecError;

use crate::specs::parse_game_spec;
use crate::logic::Game;

fn main() -> Result<(), GameSpecError> {
    let game_spec = parse_game_spec("specs/chess.json")?;
    let mut game = Game::from_spec(game_spec);
    
    println!("Parsed game: {:?}", game);

    // TEMP: Test move calculation (using knight)
    game.calculate_moves(vec![5, 0]);
    println!("Bishop @[5, 0] moves: {:?}", game.available_moves);

    game.calculate_moves(vec![1, 1]);
    println!("Pawn @[1, 1] moves: {:?}", game.available_moves);

    Ok(())
}
