use serde_json;
use std::fs;
use std::path::Path;

use super::map::map_refusal;
use crate::specs::{GameSpec, GameSpecError};

/// Parses and validates game spec contents from a file.
pub fn parse_spec<P: AsRef<Path>>(file_path: P) -> Result<GameSpec, GameSpecError> {
    let contents = fs::read_to_string(file_path)?;
    parse_spec_str(&contents)
}

/// Parses and validates game spec contents from JSON text.
pub fn parse_spec_str(contents: &str) -> Result<GameSpec, GameSpecError> {
    let game: GameSpec = serde_json::from_str(contents)
        .map_err(|error| map_refusal(&error).unwrap_or(GameSpecError::JsonError(error)))?;

    game.validate_specs()?;

    Ok(game)
}
