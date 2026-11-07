use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct GameInstanceRecord {
    pub id: Uuid,
    pub spec_id: Uuid,
    pub spec_slug: String,
    pub spec_version: i32,
    pub status: String,
    pub state_json: Value,
    pub revision: i64,
}
