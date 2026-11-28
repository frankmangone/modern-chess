use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::StoreError;
use crate::{DomainError, Engine, EngineError, GameSpec, Slug, SpecId};

const COLUMNS: &str = "id, slug, version, name, rules, rules_hash, active, created_at";

#[derive(Clone)]
pub struct SpecRepo {
    pool: PgPool,
    engine: Engine,
}

impl SpecRepo {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            engine: Engine::new(),
        }
    }

    /// The engine that reads rules before they are kept.
    pub fn with_engine(mut self, engine: Engine) -> Self {
        self.engine = engine;
        self
    }

    /// Stores `rules` as the active version of `slug`.
    ///
    /// The engine reads the rules first, and rules it will not read are refused. The same rules
    /// again change nothing. Changed rules become the next version and the previous one is kept,
    /// inactive, for matches that already use it.
    pub async fn upsert(
        &self,
        slug: Slug,
        name: String,
        rules: Value,
        now: DateTime<Utc>,
    ) -> Result<GameSpec, StoreError> {
        match self.engine.check(&rules).await {
            Ok(_) => {}
            Err(EngineError::Unusable(why)) => return Err(DomainError::InvalidRules(why).into()),
            Err(EngineError::Stopped(why)) => return Err(StoreError::Engine(why)),
        }
        let mut candidate = GameSpec::new(slug, 1, name, rules, now);

        let mut tx = self.pool.begin().await?;
        // One writer per game at a time, so two seeders cannot both take version N+1.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
            .bind(candidate.slug.as_str())
            .execute(&mut *tx)
            .await?;

        let latest = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM game_specs WHERE slug = $1 ORDER BY version DESC LIMIT 1"
        ))
        .bind(candidate.slug.as_str())
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = latest {
            let current = spec_from_row(&row)?;
            if current.rules_hash == candidate.rules_hash {
                return Ok(current);
            }
            candidate.version = current.version + 1;
        }

        sqlx::query("UPDATE game_specs SET active = FALSE WHERE slug = $1 AND active")
            .bind(candidate.slug.as_str())
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO game_specs (id, slug, version, name, rules, rules_hash, active, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, TRUE, $7)",
        )
        .bind(candidate.id.as_uuid())
        .bind(candidate.slug.as_str())
        .bind(i32::try_from(candidate.version).map_err(|e| sqlx::Error::Decode(Box::new(e)))?)
        .bind(&candidate.name)
        .bind(&candidate.rules)
        .bind(&candidate.rules_hash)
        .bind(candidate.created_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(candidate)
    }

    pub async fn get(&self, id: SpecId) -> Result<Option<GameSpec>, StoreError> {
        let row = sqlx::query(&format!("SELECT {COLUMNS} FROM game_specs WHERE id = $1"))
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(spec_from_row).transpose()
    }

    /// The newest version of a game's rules.
    pub async fn latest(&self, slug: &Slug) -> Result<Option<GameSpec>, StoreError> {
        let row = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM game_specs WHERE slug = $1 ORDER BY version DESC LIMIT 1"
        ))
        .bind(slug.as_str())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(spec_from_row).transpose()
    }
}

/// A stored spec is returned as written, even when the engine can no longer read its
/// rules: callers find that out when they use it, and can skip just that game.
fn spec_from_row(row: &PgRow) -> Result<GameSpec, StoreError> {
    let version: i32 = row.try_get("version")?;
    Ok(GameSpec {
        id: SpecId::from_uuid(row.try_get::<Uuid, _>("id")?),
        slug: Slug::parse(&row.try_get::<String, _>("slug")?)?,
        version: u32::try_from(version).map_err(|e| sqlx::Error::Decode(Box::new(e)))?,
        name: row.try_get("name")?,
        rules: row.try_get("rules")?,
        rules_hash: row.try_get("rules_hash")?,
        active: row.try_get("active")?,
        created_at: row.try_get("created_at")?,
    })
}
