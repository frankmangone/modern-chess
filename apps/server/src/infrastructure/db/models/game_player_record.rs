use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GamePlayerRecord {
    pub seat: String,
    pub user_id: String,
}
