use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SeededSpecSummary {
    pub slug: String,
    pub version: i32,
    pub name: String,
    pub rules_hash: String,
}
