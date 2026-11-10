use chrono::{DateTime, Utc};
use modern_chess_kernel::{Handle, UserId};
use serde::{Deserialize, Serialize};

/// A person using the product: a guest, or someone who has signed in.
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

    /// A user made at a first sign-in. Nothing in the user says it came from an account: the
    /// link to the provider lives in the identities table.
    pub fn new_account(handle: Handle, now: DateTime<Utc>) -> Self {
        Self::new_guest(handle, now)
    }
}
