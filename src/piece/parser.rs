use std::fs;
use std::path::Path;
use serde_json;
use crate::piece::structs::Piece;

pub fn parse_piece_spec<P: AsRef<Path>>(file_path: P) -> Result<Piece, Box<dyn std::error::Error>> {
    // Read the file contents
    let contents = fs::read_to_string(file_path)?;

    // Parse the JSON string into our Piece struct
    let piece: Piece = serde_json::from_str(&contents)?;

    Ok(piece)
}
