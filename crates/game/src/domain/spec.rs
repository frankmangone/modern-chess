use chrono::{DateTime, Utc};
use modern_chess::logic::Game;
use modern_chess::specs::{parse_game_spec_str, GameSpec as EngineSpec};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{DomainError, SeatName, Slug, SpecId};

/// One stored version of a game's rules.
///
/// Immutable once stored: changed rules are a new version, never an edit, so a match in
/// progress keeps the rules it started with. `seats()` and the opening position are read
/// through the engine's own parser, so nothing about the rules is duplicated here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameSpec {
    pub id: SpecId,
    pub slug: Slug,
    pub version: u32,
    pub name: String,
    pub rules: Value,
    pub rules_hash: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

impl GameSpec {
    /// Checks the rules with the engine's parser before accepting them.
    pub fn new(
        slug: Slug,
        version: u32,
        name: String,
        rules: Value,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        parse_rules(&rules)?;
        Ok(Self {
            id: SpecId::new(),
            slug,
            version,
            name,
            rules_hash: hash_rules(&rules),
            rules,
            active: true,
            created_at: now,
        })
    }

    /// The seats of this game, in the order the rules list their players.
    pub fn seats(&self) -> Result<Vec<SeatName>, DomainError> {
        Ok(parse_rules(&self.rules)?
            .players
            .into_iter()
            .map(|player| SeatName::new(player.name))
            .collect())
    }

    /// The engine's saved state of the opening position.
    pub(super) fn initial_state(&self) -> Result<Value, DomainError> {
        let game = Game::from_spec(parse_rules(&self.rules)?);
        let saved = game
            .save_state()
            .map_err(|err| DomainError::InvalidRules(err.to_string()))?;
        serde_json::from_str(&saved).map_err(|err| DomainError::InvalidRules(err.to_string()))
    }
}

fn parse_rules(rules: &Value) -> Result<EngineSpec, DomainError> {
    parse_game_spec_str(&rules.to_string())
        .map_err(|err| DomainError::InvalidRules(err.to_string()))
}

fn hash_rules(rules: &Value) -> String {
    hex::encode(Sha256::digest(rules.to_string().as_bytes()))
}
