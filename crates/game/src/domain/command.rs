use chrono::{DateTime, Utc};
use modern_chess_kernel::UserId;
use serde::{Deserialize, Serialize};

use super::InstanceId;

/// The longest position name the API accepts. Names are short; the log should not keep whatever
/// a client sends.
pub const MAX_POSITION_BYTES: usize = 64;

/// What a player asked the game to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandKind {
    Select {
        position: String,
    },
    Move {
        position: String,
    },
    Promote {
        code: String,
    },
    Drop {
        code: String,
        position: String,
    },
    Pass,
    /// Give the match up. Either seat may ask at any time while it is being played.
    Resign,
}

impl CommandKind {
    /// The square the command is about, for the commands that name one.
    pub fn position(&self) -> Option<&str> {
        match self {
            Self::Select { position } | Self::Move { position } | Self::Drop { position, .. } => {
                Some(position)
            }
            Self::Promote { .. } | Self::Pass | Self::Resign => None,
        }
    }
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
