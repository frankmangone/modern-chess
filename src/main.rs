mod specs;
mod structs;

use crate::specs::{parse_piece_spec, parse_game_spec};
use crate::structs::Game;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game = parse_game_spec("specs/games/chess.json")?;
    let game = Game::from_spec(game);
    println!("Parsed game: {:?}", game);

    let pawn = parse_piece_spec("specs/pieces/pawn.json")?;
    println!("Parsed pawn: {:?}", pawn);

    let king = parse_piece_spec("specs/pieces/king.json")?;
    println!("Parsed king: {:?}", king);

    Ok(())
}
