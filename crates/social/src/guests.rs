//! Signing up as a guest: a new user with a generated handle like `Guest-4821`.

use chrono::{DateTime, Utc};
use modern_chess_kernel::Handle;
use thiserror::Error;
use uuid::Uuid;

use crate::store::{StoreError, UserRepo};
use crate::User;

/// Four-digit numbers first; if they keep clashing, six digits leave far more room.
const ATTEMPTS: [(usize, u32); 2] = [(5, 4), (5, 6)];

#[derive(Debug, Error)]
pub enum GuestError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("no free guest handle was found")]
    NoHandleAvailable,
}

/// Creates and stores a guest. `number` supplies the digits for a handle (it is told how
/// many); a clash with an existing handle is retried with a fresh number.
pub async fn create_guest(
    users: &UserRepo,
    now: DateTime<Utc>,
    mut number: impl FnMut(u32) -> u32,
) -> Result<User, GuestError> {
    for (tries, digits) in ATTEMPTS {
        for _ in 0..tries {
            let handle = Handle::parse(&format!("Guest-{}", number(digits)))
                .expect("Guest-<digits> is always a valid handle");
            let user = User::new_guest(handle, now);
            match users.insert(&user).await {
                Ok(()) => return Ok(user),
                Err(StoreError::HandleTaken) => continue,
                Err(other) => return Err(other.into()),
            }
        }
    }
    Err(GuestError::NoHandleAvailable)
}

/// A random number with exactly `digits` digits.
pub fn random_number(digits: u32) -> u32 {
    let low = 10u128.pow(digits - 1);
    let span = 9 * low;
    (low + Uuid::new_v4().as_u128() % span) as u32
}
