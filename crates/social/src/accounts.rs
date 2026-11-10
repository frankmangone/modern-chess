//! Signing in with a provider identity, and deleting an account.
//!
//! The rule that matters: a provider's user id is only ever looked up here. Everything else in
//! the product knows a person by our own [`UserId`].

use chrono::{DateTime, Utc};
use modern_chess_kernel::{Handle, UserId};
use thiserror::Error;

use crate::guests::ATTEMPTS;
use crate::provider::ExternalIdentity;
use crate::store::{AccountRepo, StoreError, UserRepo};
use crate::User;

#[derive(Debug, Error)]
pub enum AccountError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("no free handle was found")]
    NoHandleAvailable,
    #[error("an identity points at a user that does not exist")]
    DanglingIdentity,
}

/// The user for a provider identity.
///
/// - An identity that is already linked signs in as its user.
/// - Otherwise, a live `guest` with no identity from this provider is adopted: the identity is
///   linked to the guest's own user, so everything the guest did stays theirs. A guest that is
///   itself an account is never adopted, so a signed-in user's token cannot be used to attach
///   a second identity to them.
/// - Otherwise a new user is made, with a generated handle like `Player-4821`.
///
/// `number` supplies the digits for a handle (it is told how many).
pub async fn sign_in(
    users: &UserRepo,
    accounts: &AccountRepo,
    identity: &ExternalIdentity,
    guest: Option<UserId>,
    now: DateTime<Utc>,
    mut number: impl FnMut(u32) -> u32,
) -> Result<User, AccountError> {
    if let Some(user) = linked_user(users, accounts, identity).await? {
        return Ok(user);
    }

    if let Some(guest) = guest {
        if let Some(user) = users.get(guest).await? {
            match accounts.link(user.id, identity, now).await {
                Ok(()) => return Ok(user),
                // Someone linked this identity a moment ago: they are the user.
                Err(StoreError::IdentityTaken) => {
                    if let Some(user) = linked_user(users, accounts, identity).await? {
                        return Ok(user);
                    }
                }
                // The guest was a signed-in account, or was deleted just now: not adoptable.
                Err(StoreError::UserAlreadyLinked | StoreError::UserMissing) => {}
                Err(other) => return Err(other.into()),
            }
        }
    }

    create_account(users, accounts, identity, now, &mut number).await
}

/// Deletes the account of `user`. False when there is no such live user.
pub async fn delete_account(
    accounts: &AccountRepo,
    user: UserId,
    now: DateTime<Utc>,
) -> Result<bool, StoreError> {
    accounts.delete(user, now).await
}

async fn linked_user(
    users: &UserRepo,
    accounts: &AccountRepo,
    identity: &ExternalIdentity,
) -> Result<Option<User>, AccountError> {
    let Some(id) = accounts
        .find_user(&identity.provider, &identity.subject)
        .await?
    else {
        return Ok(None);
    };
    users
        .get(id)
        .await?
        .map(Some)
        .ok_or(AccountError::DanglingIdentity)
}

async fn create_account(
    users: &UserRepo,
    accounts: &AccountRepo,
    identity: &ExternalIdentity,
    now: DateTime<Utc>,
    number: &mut impl FnMut(u32) -> u32,
) -> Result<User, AccountError> {
    for (tries, digits) in ATTEMPTS {
        for _ in 0..tries {
            let handle = Handle::parse(&format!("Player-{}", number(digits)))
                .expect("Player-<digits> is always a valid handle");
            let user = User::new_account(handle, now);
            match accounts.create_with_identity(&user, identity, now).await {
                Ok(()) => return Ok(user),
                Err(StoreError::HandleTaken) => continue,
                // Lost the race to make this account: the winner's user is the answer.
                Err(StoreError::IdentityTaken) => {
                    return linked_user(users, accounts, identity)
                        .await?
                        .ok_or(AccountError::DanglingIdentity)
                }
                Err(other) => return Err(other.into()),
            }
        }
    }
    Err(AccountError::NoHandleAvailable)
}
