use serde_json;
use std::fs;
use std::path::Path;

use crate::specs::{GameSpec, GameSpecError};

/// Parses and validates game spec contents from a file.
pub fn parse_spec<P: AsRef<Path>>(file_path: P) -> Result<GameSpec, GameSpecError> {
    let contents = fs::read_to_string(file_path)?;
    parse_spec_str(&contents)
}

/// Parses and validates game spec contents from JSON text.
pub fn parse_spec_str(contents: &str) -> Result<GameSpec, GameSpecError> {
    let game: GameSpec = serde_json::from_str(contents)?;

    game.validate_specs()?;

    Ok(game)
}
