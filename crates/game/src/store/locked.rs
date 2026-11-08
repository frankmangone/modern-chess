use chrono::{DateTime, Utc};
use modern_chess_kernel::UserId;
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{Postgres, Transaction};

use super::command::append_locked;
use super::instance::{status_to_db, to_i64};
use super::StoreError;
use crate::{CommandKind, GameInstance, Outcome};

/// A match whose row is locked by an open transaction: the one place where a move is decided
/// and stored together. Finish it with [`accept`](Self::accept) or [`reject`](Self::reject);
/// dropping it rolls everything back and leaves the match and its log untouched.
pub struct LockedInstance {
    tx: Transaction<'static, Postgres>,
    instance: GameInstance,
}

impl LockedInstance {
    pub(super) fn new(tx: Transaction<'static, Postgres>, instance: GameInstance) -> Self {
        Self { tx, instance }
    }

    /// The match as it is, and as nobody else can change it while this is held.
    pub fn instance(&self) -> &GameInstance {
        &self.instance
    }

    /// Stores the position a command led to, bumps the revision, finishes the match if
    /// `outcome` is set, and logs the command as accepted. All or nothing.
    pub async fn accept(
        mut self,
        actor: UserId,
        kind: CommandKind,
        state: Value,
        outcome: Option<Outcome>,
        now: DateTime<Utc>,
    ) -> Result<GameInstance, StoreError> {
        self.instance.record_move(state, outcome, now)?;
        sqlx::query(
            "UPDATE game_instances \
             SET state = $2, revision = $3, status = $4, outcome = $5, finished_at = $6 \
             WHERE id = $1",
        )
        .bind(self.instance.id().as_uuid())
        .bind(self.instance.state())
        .bind(to_i64(self.instance.revision())?)
        .bind(status_to_db(self.instance.status()))
        .bind(self.instance.outcome().map(Json))
        .bind(self.instance.finished_at())
        .execute(&mut *self.tx)
        .await?;
        append_locked(
            &mut self.tx,
            self.instance.id(),
            actor,
            kind,
            true,
            None,
            now,
        )
        .await?;
        self.tx.commit().await?;
        Ok(self.instance)
    }

    /// Logs a command the rules refused. The match itself does not change.
    pub async fn reject(
        mut self,
        actor: UserId,
        kind: CommandKind,
        reason: String,
        now: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        append_locked(
            &mut self.tx,
            self.instance.id(),
            actor,
            kind,
            false,
            Some(reason),
            now,
        )
        .await?;
        self.tx.commit().await?;
        Ok(())
    }
}
