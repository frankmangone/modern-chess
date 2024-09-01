use std::fs;
use std::path::Path;
use serde_json;

use crate::specs::{GameSpec, GameSpecError};

/// Parses and validates game spec contents.
pub fn parse_spec<P: AsRef<Path>>(file_path: P) -> Result<GameSpec, GameSpecError> {
    let contents = fs::read_to_string(file_path)?;
    let game: GameSpec = serde_json::from_str(&contents)?;

    game.validate()?;

    Ok(game)
}
