use chrono::{DateTime, Utc};
use modern_chess_kernel::{Handle, UserId};
use serde::{Deserialize, Serialize};

/// A person using the product. In v1 every user is a guest; accounts come later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: UserId,
    pub handle: Handle,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn new_guest(handle: Handle, now: DateTime<Utc>) -> Self {
        Self {
            id: UserId::new(),
            handle,
            created_at: now,
        }
    }
}
