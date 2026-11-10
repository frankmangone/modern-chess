use chrono::{DateTime, Utc};
use modern_chess_kernel::UserId;
use sqlx::PgPool;
use uuid::Uuid;

use super::user::insert_on;
use super::StoreError;
use crate::provider::ExternalIdentity;
use crate::User;

const IDENTITY_KEY: &str = "pk_identities";
const ONE_PER_PROVIDER: &str = "uq_identities_user_provider";
const USER_REFERENCE: &str = "fk_identities_user";

/// Provider identities, and the writes that must change them together with `users`.
#[derive(Clone)]
pub struct AccountRepo {
    pool: PgPool,
}

impl AccountRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// The user an identity is linked to, if it is linked.
    pub async fn find_user(
        &self,
        provider: &str,
        subject: &str,
    ) -> Result<Option<UserId>, StoreError> {
        let id: Option<Uuid> = sqlx::query_scalar(
            "SELECT user_id FROM identities WHERE provider = $1 AND subject = $2",
        )
        .bind(provider)
        .bind(subject)
        .fetch_optional(&self.pool)
        .await?;
        Ok(id.map(UserId::from_uuid))
    }

    /// Links an identity to an existing, live user. Refused when the identity already belongs to
    /// someone (`IdentityTaken`), the user already has an identity from this provider
    /// (`UserAlreadyLinked`), or the user does not exist or has been deleted (`UserMissing`).
    ///
    /// The user's row is locked while the link is made, so a delete that is running at the same
    /// moment either finishes first (and this is refused) or waits (and then removes the new
    /// link along with the rest). The foreign key alone cannot do this: deleted rows still exist.
    pub async fn link(
        &self,
        user: UserId,
        identity: &ExternalIdentity,
        now: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await?;
        let live: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM users WHERE id = $1 AND deleted_at IS NULL FOR SHARE",
        )
        .bind(user.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        if live.is_none() {
            return Err(StoreError::UserMissing);
        }
        insert_identity(&mut *tx, user, identity, now).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Stores a new user and links the identity to it, or neither. A handle clash is reported
    /// as `HandleTaken` and a lost race for the identity as `IdentityTaken`.
    pub async fn create_with_identity(
        &self,
        user: &User,
        identity: &ExternalIdentity,
        now: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await?;
        insert_on(&mut *tx, user).await?;
        insert_identity(&mut *tx, user.id, identity, now).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Deletes an account: its identities go, its handle becomes `deleted-<digits>`, and it no
    /// longer counts as a user. The row stays, because finished matches still point at it.
    /// Returns false when there was no such live user.
    pub async fn delete(&self, user: UserId, now: DateTime<Utc>) -> Result<bool, StoreError> {
        let handle = format!("deleted-{}", &user.as_uuid().simple().to_string()[..12]);
        let mut tx = self.pool.begin().await?;
        let updated = sqlx::query(
            "UPDATE users SET handle = $2, deleted_at = $3 WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(user.as_uuid())
        .bind(&handle)
        .bind(now)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if updated == 0 {
            return Ok(false);
        }
        sqlx::query("DELETE FROM identities WHERE user_id = $1")
            .bind(user.as_uuid())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(true)
    }
}

async fn insert_identity<'e>(
    executor: impl sqlx::PgExecutor<'e>,
    user: UserId,
    identity: &ExternalIdentity,
    now: DateTime<Utc>,
) -> Result<(), StoreError> {
    let result = sqlx::query(
        "INSERT INTO identities (provider, subject, user_id, email, created_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&identity.provider)
    .bind(&identity.subject)
    .bind(user.as_uuid())
    .bind(identity.email.as_deref())
    .bind(now)
    .execute(executor)
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(sqlx::Error::Database(err)) => match err.constraint() {
            Some(IDENTITY_KEY) => Err(StoreError::IdentityTaken),
            Some(ONE_PER_PROVIDER) => Err(StoreError::UserAlreadyLinked),
            Some(USER_REFERENCE) => Err(StoreError::UserMissing),
            _ => Err(sqlx::Error::Database(err).into()),
        },
        Err(other) => Err(other.into()),
    }
}
