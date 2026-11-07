use chrono::{DateTime, Utc};
use modern_chess_kernel::UserId;
use serde::{Deserialize, Serialize};

use super::InstanceId;

/// What a player asked the game to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandKind {
    Select { x: u8, y: u8 },
    Move { x: u8, y: u8 },
    Promote { code: String },
    Drop { code: String, x: u8, y: u8 },
}

/// One line of the append-only log of requests, accepted or not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    pub instance_id: InstanceId,
    pub seq: u64,
    pub actor: UserId,
    pub kind: CommandKind,
    pub accepted: bool,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}
