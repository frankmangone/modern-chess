use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct StoredGameSpec {
    pub id: Uuid,
    pub slug: String,
    pub version: i32,
    pub name: String,
    pub rules_json: Value,
    pub rules_hash: String,
    pub is_active: bool,
}
