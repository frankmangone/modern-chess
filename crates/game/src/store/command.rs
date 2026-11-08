use chrono::{DateTime, Utc};
use modern_chess_kernel::UserId;
use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{PgConnection, PgPool, Row};
use uuid::Uuid;

use super::StoreError;
use crate::{Command, CommandKind, InstanceId};

#[derive(Clone)]
pub struct CommandRepo {
    pool: PgPool,
}

impl CommandRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Appends to an instance's log and returns the stored line. Sequence numbers start at 1
    /// and have no gaps: the instance row is locked while the next number is taken.
    pub async fn append(
        &self,
        instance: InstanceId,
        actor: UserId,
        kind: CommandKind,
        accepted: bool,
        error: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<Command, StoreError> {
        let mut tx = self.pool.begin().await?;
        let locked = sqlx::query("SELECT 1 FROM game_instances WHERE id = $1 FOR UPDATE")
            .bind(instance.as_uuid())
            .fetch_optional(&mut *tx)
            .await?;
        if locked.is_none() {
            return Err(StoreError::NotFound);
        }
        let command = append_locked(&mut tx, instance, actor, kind, accepted, error, now).await?;
        tx.commit().await?;
        Ok(command)
    }

    /// An instance's log, oldest first.
    pub async fn list(&self, instance: InstanceId) -> Result<Vec<Command>, StoreError> {
        let rows = sqlx::query(
            "SELECT instance_id, seq, actor, kind, accepted, error, created_at \
             FROM game_commands WHERE instance_id = $1 ORDER BY seq",
        )
        .bind(instance.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(command_from_row).collect()
    }
}

/// Appends inside a transaction that already holds the instance row's lock.
pub(super) async fn append_locked(
    conn: &mut PgConnection,
    instance: InstanceId,
    actor: UserId,
    kind: CommandKind,
    accepted: bool,
    error: Option<String>,
    now: DateTime<Utc>,
) -> Result<Command, StoreError> {
    let seq: i64 = sqlx::query_scalar(
        "INSERT INTO game_commands (instance_id, seq, actor, kind, accepted, error, created_at) \
         SELECT $1, COALESCE(MAX(seq), 0) + 1, $2, $3, $4, $5, $6 \
         FROM game_commands WHERE instance_id = $1 \
         RETURNING seq",
    )
    .bind(instance.as_uuid())
    .bind(actor.as_uuid())
    .bind(Json(&kind))
    .bind(accepted)
    .bind(&error)
    .bind(now)
    .fetch_one(conn)
    .await?;

    Ok(Command {
        instance_id: instance,
        seq: u64::try_from(seq).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        actor,
        kind,
        accepted,
        error,
        created_at: now,
    })
}

fn command_from_row(row: &PgRow) -> Result<Command, StoreError> {
    Ok(Command {
        instance_id: InstanceId::from_uuid(row.try_get::<Uuid, _>("instance_id")?),
        seq: u64::try_from(row.try_get::<i64, _>("seq")?)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        actor: UserId::from_uuid(row.try_get::<Uuid, _>("actor")?),
        kind: row.try_get::<Json<CommandKind>, _>("kind")?.0,
        accepted: row.try_get("accepted")?,
        error: row.try_get("error")?,
        created_at: row.try_get("created_at")?,
    })
}
