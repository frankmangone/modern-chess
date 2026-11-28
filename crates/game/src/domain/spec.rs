use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{Slug, SpecId};

/// One stored version of a game's rules.
///
/// Immutable once stored: changed rules are a new version, never an edit, so a match in
/// progress keeps the rules it started with. What the rules mean (seats, the opening position)
/// is the engine's to say (`crate::Engine`), so nothing about them is duplicated here.
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
    /// A new version of a game's rules. The caller has already had the engine read them
    /// (`Engine::check`): nothing here runs a game's rules.
    pub fn new(slug: Slug, version: u32, name: String, rules: Value, now: DateTime<Utc>) -> Self {
        Self {
            id: SpecId::new(),
            slug,
            version,
            name,
            rules_hash: hash_rules(&rules),
            rules,
            active: true,
            created_at: now,
        }
    }
}

fn hash_rules(rules: &Value) -> String {
    hex::encode(Sha256::digest(rules.to_string().as_bytes()))
}
