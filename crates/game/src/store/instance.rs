use std::collections::HashMap;

use chrono::{DateTime, Utc};
use modern_chess_kernel::{Handle, UserId};
use sqlx::postgres::PgRow;
use sqlx::types::Json;
use sqlx::{PgConnection, PgPool, Row};
use tracing::warn;
use uuid::Uuid;

use super::StoreError;
use crate::{
    DomainError, GameInstance, InstanceId, InstanceParts, InstanceStatus, Outcome, Player,
    SeatName, Slug, SpecId,
};

const COLUMNS: &str = "id, spec_id, slug, spec_version, status, seats, state, revision, outcome, \
                       created_at, started_at, finished_at";

#[derive(Clone)]
pub struct InstanceRepo {
    pool: PgPool,
}

impl InstanceRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Stores a new instance together with any players it already has.
    pub async fn create(&self, instance: &GameInstance) -> Result<(), StoreError> {
        let parts = instance.to_parts();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO game_instances \
                 (id, spec_id, slug, spec_version, status, seats, state, revision, outcome, \
                  created_at, started_at, finished_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(parts.id.as_uuid())
        .bind(parts.spec_id.as_uuid())
        .bind(parts.slug.as_str())
        .bind(to_i32(parts.spec_version)?)
        .bind(status_to_db(parts.status))
        .bind(
            parts
                .seats
                .iter()
                .map(|s| s.as_str().to_string())
                .collect::<Vec<_>>(),
        )
        .bind(&parts.state)
        .bind(to_i64(parts.revision)?)
        .bind(parts.outcome.as_ref().map(Json))
        .bind(parts.created_at)
        .bind(parts.started_at)
        .bind(parts.finished_at)
        .execute(&mut *tx)
        .await?;
        for player in &parts.players {
            insert_player(&mut tx, parts.id, player).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn get(&self, id: InstanceId) -> Result<Option<GameInstance>, StoreError> {
        let mut conn = self.pool.acquire().await?;
        load(&mut conn, id, false).await
    }

    /// Lobbies of `slug` still waiting for players and created after `not_before`, newest
    /// first. A stored row that no longer makes sense is logged and left out of the list.
    pub async fn open_lobbies(
        &self,
        slug: &Slug,
        not_before: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<GameInstance>, StoreError> {
        let rows = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM game_instances \
             WHERE slug = $1 AND status = 'LOBBY' AND created_at > $2 \
             ORDER BY created_at DESC LIMIT $3"
        ))
        .bind(slug.as_str())
        .bind(not_before)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let ids: Vec<Uuid> = rows.iter().map(|r| r.get::<Uuid, _>("id")).collect();
        let player_rows = sqlx::query(
            "SELECT instance_id, seat, user_id, handle, joined_at FROM game_players \
             WHERE instance_id = ANY($1) ORDER BY joined_at, seat",
        )
        .bind(&ids)
        .fetch_all(&self.pool)
        .await?;
        let mut players: HashMap<Uuid, Vec<Player>> = HashMap::new();
        for row in &player_rows {
            players
                .entry(row.get("instance_id"))
                .or_default()
                .push(player_from_row(row)?);
        }

        let mut lobbies = Vec::with_capacity(rows.len());
        for row in &rows {
            let id: Uuid = row.get("id");
            let parts = parts_from_row(row, players.remove(&id).unwrap_or_default())?;
            match GameInstance::restore(parts) {
                Ok(lobby) => lobbies.push(lobby),
                Err(err) => warn!("lobby {id} is not usable and is left out: {err}"),
            }
        }
        Ok(lobbies)
    }

    /// Seats `user` at `seat`.
    ///
    /// The instance row is locked for the whole transaction, so racing joins are decided one
    /// after the other by the domain rules: for one seat exactly one wins and the rest get
    /// `SeatTaken`, and taking the last seat starts the game exactly once.
    pub async fn join(
        &self,
        id: InstanceId,
        user: UserId,
        handle: Handle,
        seat: SeatName,
        now: DateTime<Utc>,
    ) -> Result<GameInstance, StoreError> {
        let mut tx = self.pool.begin().await?;
        let mut instance = load(&mut tx, id, true).await?.ok_or(StoreError::NotFound)?;

        instance.join(user, handle, seat, now)?;

        let joined = instance
            .players()
            .last()
            .expect("a successful join adds a player")
            .clone();
        insert_player(&mut tx, id, &joined).await?;
        sqlx::query("UPDATE game_instances SET status = $2, started_at = $3 WHERE id = $1")
            .bind(id.as_uuid())
            .bind(status_to_db(instance.status()))
            .bind(instance.started_at())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;

        Ok(instance)
    }
}

async fn insert_player(
    conn: &mut PgConnection,
    instance: InstanceId,
    player: &Player,
) -> Result<(), StoreError> {
    sqlx::query(
        "INSERT INTO game_players (instance_id, seat, user_id, handle, joined_at) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(instance.as_uuid())
    .bind(player.seat.as_str())
    .bind(player.user.as_uuid())
    .bind(player.handle.as_str())
    .bind(player.joined_at)
    .execute(conn)
    .await?;
    Ok(())
}

async fn load(
    conn: &mut PgConnection,
    id: InstanceId,
    lock: bool,
) -> Result<Option<GameInstance>, StoreError> {
    let suffix = if lock { " FOR UPDATE" } else { "" };
    let row = sqlx::query(&format!(
        "SELECT {COLUMNS} FROM game_instances WHERE id = $1{suffix}"
    ))
    .bind(id.as_uuid())
    .fetch_optional(&mut *conn)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };

    let player_rows = sqlx::query(
        "SELECT seat, user_id, handle, joined_at FROM game_players \
         WHERE instance_id = $1 ORDER BY joined_at, seat",
    )
    .bind(id.as_uuid())
    .fetch_all(&mut *conn)
    .await?;
    let players = player_rows
        .iter()
        .map(player_from_row)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(GameInstance::restore(parts_from_row(&row, players)?)?))
}

fn parts_from_row(row: &PgRow, players: Vec<Player>) -> Result<InstanceParts, StoreError> {
    let seats: Vec<String> = row.try_get("seats")?;
    let outcome: Option<Json<Outcome>> = row.try_get("outcome")?;
    Ok(InstanceParts {
        id: InstanceId::from_uuid(row.try_get::<Uuid, _>("id")?),
        spec_id: SpecId::from_uuid(row.try_get::<Uuid, _>("spec_id")?),
        slug: Slug::parse(&row.try_get::<String, _>("slug")?)?,
        spec_version: u32::try_from(row.try_get::<i32, _>("spec_version")?)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        status: status_from_db(&row.try_get::<String, _>("status")?)?,
        seats: seats.into_iter().map(SeatName::new).collect(),
        players,
        state: row.try_get("state")?,
        revision: u64::try_from(row.try_get::<i64, _>("revision")?)
            .map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        outcome: outcome.map(|json| json.0),
        created_at: row.try_get("created_at")?,
        started_at: row.try_get("started_at")?,
        finished_at: row.try_get("finished_at")?,
    })
}

fn player_from_row(row: &PgRow) -> Result<Player, StoreError> {
    Ok(Player {
        seat: SeatName::new(row.try_get::<String, _>("seat")?),
        user: UserId::from_uuid(row.try_get::<Uuid, _>("user_id")?),
        handle: Handle::parse(&row.try_get::<String, _>("handle")?).map_err(|err| {
            StoreError::Domain(DomainError::InvalidInstance(format!(
                "stored handle is invalid: {err}"
            )))
        })?,
        joined_at: row.try_get("joined_at")?,
    })
}

/// The database spells statuses in capitals, the JSON API in lowercase; this is the only
/// place that knows the database's spelling.
fn status_to_db(status: InstanceStatus) -> &'static str {
    match status {
        InstanceStatus::Lobby => "LOBBY",
        InstanceStatus::Active => "ACTIVE",
        InstanceStatus::Finished => "FINISHED",
    }
}

fn status_from_db(raw: &str) -> Result<InstanceStatus, StoreError> {
    match raw {
        "LOBBY" => Ok(InstanceStatus::Lobby),
        "ACTIVE" => Ok(InstanceStatus::Active),
        "FINISHED" => Ok(InstanceStatus::Finished),
        other => Err(DomainError::InvalidInstance(format!("unknown status {other:?}")).into()),
    }
}

fn to_i32(value: u32) -> Result<i32, sqlx::Error> {
    i32::try_from(value).map_err(|e| sqlx::Error::Encode(Box::new(e)))
}

fn to_i64(value: u64) -> Result<i64, sqlx::Error> {
    i64::try_from(value).map_err(|e| sqlx::Error::Encode(Box::new(e)))
}
