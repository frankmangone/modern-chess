use serde_json;
use std::fs;
use std::path::Path;

use super::game::FORMAT_VERSION;
use super::map::map_refusal;
use crate::specs::{GameSpec, GameSpecError};

/// Parses and validates game spec contents from a file.
pub fn parse_spec<P: AsRef<Path>>(file_path: P) -> Result<GameSpec, GameSpecError> {
    let contents = fs::read_to_string(file_path)?;
    parse_spec_str(&contents)
}

/// Parses and validates game spec contents from JSON text.
pub fn parse_spec_str(contents: &str) -> Result<GameSpec, GameSpecError> {
    check_format_version(contents)?;

    let game: GameSpec = serde_json::from_str(contents)
        .map_err(|error| map_refusal(&error).unwrap_or(GameSpecError::JsonError(error)))?;

    game.validate_specs()?;

    Ok(game)
}

/// Refuses a spec that does not say it is written for `FORMAT_VERSION`, before the rest is read:
/// a spec for a newer format is told so, not told about its keys one by one. Text that is not a
/// JSON object is left to the full read, which gives its own error.
fn check_format_version(contents: &str) -> Result<(), GameSpecError> {
    let Ok(serde_json::Value::Object(fields)) = serde_json::from_str(contents) else {
        return Ok(());
    };
    match fields.get("format_version") {
        None => Err(GameSpecError::MissingFormatVersion),
        Some(version) if version.as_u64() == Some(u64::from(FORMAT_VERSION)) => Ok(()),
        Some(version) => Err(GameSpecError::UnsupportedFormatVersion(version.to_string())),
    }
}
