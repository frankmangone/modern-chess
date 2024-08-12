mod piece;
mod specs;

use crate::piece::parser::parse_piece_spec;
use crate::piece::structs::Piece;

use crate::specs::parse_game_spec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game = parse_game_spec("specs/games/chess.json");
    println!("Parsed game: {:?}", game);
    Ok(())
}

// fn parse_chess_pieces() -> Result<Vec<Piece>, Box<dyn std::error::Error>> {
//     let pawn: Piece = parse_piece_spec("specs/pawn.json")?;
//     println!("Parsed pawn: {:?}", pawn);

//     let rook: Piece = parse_piece_spec("specs/rook.json")?;
//     println!("Parsed rook: {:?}", rook);

//     let knight: Piece = parse_piece_spec("specs/knight.json")?;
//     println!("Parsed knight: {:?}", knight);

//     let bishop: Piece = parse_piece_spec("specs/bishop.json")?;
//     println!("Parsed bishop: {:?}", bishop);

//     let queen: Piece = parse_piece_spec("specs/queen.json")?;
//     println!("Parsed queen: {:?}", queen);

//     let king: Piece = parse_piece_spec("specs/king.json")?;
//     println!("Parsed king: {:?}", king);

//     Ok(vec![pawn, rook, knight, bishop, queen, king])
// }