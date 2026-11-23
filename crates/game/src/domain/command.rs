use chrono::{DateTime, Utc};
use modern_chess_kernel::UserId;
use serde::{Deserialize, Serialize};

use super::InstanceId;

/// The longest string the API accepts in an answer. Names are short; the log should not keep
/// whatever a client sends.
pub const MAX_POSITION_BYTES: usize = 64;

/// What a player asked the game to do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandKind {
    /// Answers the next choice of the turn with exactly one of a place, a code, a piece held in a bag
    /// or an action's name. (Logs from before bags call it `hand`; it is still read.) Logged drops from before this command carry a held piece and a place
    /// together: they are read, but the API never accepts that.
    Choose {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        place: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        code: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none", alias = "hand")]
        held: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        action: Option<String>,
    },
    /// Takes back the answers given so far this turn.
    Cancel,
    /// Give the match up. Either seat may ask at any time while it is being played.
    Resign,
}

impl CommandKind {
    /// Why the API refuses this command before logging it, if it does: a choice that does not
    /// give exactly one answer, or an answer that is too long.
    pub fn refusal(&self) -> Option<&'static str> {
        let Self::Choose {
            place,
            code,
            held,
            action,
        } = self
        else {
            return None;
        };
        let given: Vec<&String> = [place, code, held, action].into_iter().flatten().collect();
        if given.len() != 1 {
            return Some("a choice needs exactly one answer");
        }
        if given.iter().any(|answer| answer.len() > MAX_POSITION_BYTES) {
            return Some("that answer is too long");
        }
        None
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
