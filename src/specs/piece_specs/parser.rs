use std::fs;
use std::path::Path;
use serde_json;

use crate::specs::PieceSpec;

pub fn parse_spec<P: AsRef<Path>>(file_path: P) -> Result<PieceSpec, Box<dyn std::error::Error>> {
    // Read the file contents
    let contents = fs::read_to_string(file_path)?;

    // Parse the JSON string into our PieceSpec struct
    let piece: PieceSpec = serde_json::from_str(&contents)?;

    Ok(piece)
}
