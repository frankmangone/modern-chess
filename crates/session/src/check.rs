//! Telling a person writing a spec what is wrong with it. `Session::new` reports a refusal as
//! debug text, which names the engine's variant; a builder wants words and a place to point at.

use modern_chess::specs::{parse_game_spec_str, GameSpec, GameSpecError};

/// What a good spec is, in a line: enough for the builder to say "this is Chess, for two".
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SpecSummary {
    pub name: String,
    pub players: Vec<String>,
}

/// Why the engine will not read a spec. `line`, `column` (counting from 1) and `path` (`board`,
/// `pieces[2].moves[0]`) are given when the text itself is at fault; a rule that contradicts
/// another part of the spec has none, and its message names the culprit.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SpecProblem {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub path: Option<String>,
}

/// Reads `spec_json` the way a game would be opened and says what it is, or what is wrong.
pub fn check_spec(spec_json: &str) -> Result<SpecSummary, SpecProblem> {
    match parse_game_spec_str(spec_json) {
        Ok(spec) => Ok(SpecSummary {
            name: spec.name,
            players: spec.players.into_iter().map(|p| p.name).collect(),
        }),
        Err(GameSpecError::JsonError(error)) => Err(SpecProblem {
            message: without_place(&error.to_string()),
            line: Some(error.line()).filter(|line| *line > 0),
            column: Some(error.column()).filter(|column| *column > 0),
            path: path_of(spec_json),
        }),
        Err(other) => Err(SpecProblem {
            message: other.to_string(),
            line: None,
            column: None,
            path: None,
        }),
    }
}

/// serde's "… at line 3 column 5": the place is reported on its own, so it is cut from the words.
fn without_place(message: &str) -> String {
    message
        .rsplit_once(" at line ")
        .map_or(message, |(words, _)| words)
        .to_string()
}

/// Where in the spec the reader stopped, as the key and index chain that leads there. Nothing
/// when it stopped at the top, or when the text is not even JSON.
fn path_of(spec_json: &str) -> Option<String> {
    let mut reader = serde_json::Deserializer::from_str(spec_json);
    let error = serde_path_to_error::deserialize::<_, GameSpec>(&mut reader).err()?;
    let path = error.path().to_string();
    (path != ".").then_some(path)
}
