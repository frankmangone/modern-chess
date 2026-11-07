use std::fs;

use hex::encode;
use modern_chess::specs::GameSpec;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::infrastructure::db::{
    models::{GameSpecSummary, SeededSpecSummary, StoredGameSpec},
    DbError,
};

const DEFAULT_SPECS: [(&str, &str, i32); 2] = [
    (
        "chess",
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/engine/specs/chess.json"
        ),
        1,
    ),
    (
        "shogi",
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../crates/engine/specs/shogi.json"
        ),
        1,
    ),
];

#[derive(Debug, Clone)]
pub struct SpecsRepository {
    pool: PgPool,
}

impl SpecsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_specs(&self) -> Result<Vec<GameSpecSummary>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT slug, version, name, rules_hash, is_active
            FROM game_specs
            ORDER BY slug ASC, version DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(GameSpecSummary {
                    slug: row.try_get("slug")?,
                    version: row.try_get("version")?,
                    name: row.try_get("name")?,
                    rules_hash: row.try_get("rules_hash")?,
                    is_active: row.try_get("is_active")?,
                })
            })
            .collect()
    }

    pub async fn resolve_spec(
        &self,
        slug: &str,
        version: Option<i32>,
    ) -> Result<Option<StoredGameSpec>, DbError> {
        let row = if let Some(version) = version {
            sqlx::query(
                r#"
                SELECT id, slug, version, name, rules_json, rules_hash, is_active
                FROM game_specs
                WHERE slug = $1 AND version = $2
                LIMIT 1
                "#,
            )
            .bind(slug)
            .bind(version)
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query(
                r#"
                SELECT id, slug, version, name, rules_json, rules_hash, is_active
                FROM game_specs
                WHERE slug = $1 AND is_active = true
                ORDER BY version DESC
                LIMIT 1
                "#,
            )
            .bind(slug)
            .fetch_optional(&self.pool)
            .await?
        };

        row.map(map_row_to_spec).transpose()
    }
}

pub async fn seed_default_specs(pool: &PgPool) -> Result<Vec<SeededSpecSummary>, DbError> {
    let mut seeded = Vec::with_capacity(DEFAULT_SPECS.len());

    for (slug, path, version) in DEFAULT_SPECS {
        let (name, rules_json, rules_hash) = load_and_validate_spec(path)?;

        let row = sqlx::query(
            r#"
            INSERT INTO game_specs (id, slug, version, name, rules_json, rules_hash, is_active)
            VALUES ($1, $2, $3, $4, $5, $6, true)
            ON CONFLICT (slug, version)
            DO UPDATE SET
                name = EXCLUDED.name,
                rules_json = EXCLUDED.rules_json,
                rules_hash = EXCLUDED.rules_hash,
                is_active = true
            RETURNING slug, version, name, rules_hash
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(slug)
        .bind(version)
        .bind(name)
        .bind(rules_json)
        .bind(rules_hash)
        .fetch_one(pool)
        .await?;

        seeded.push(SeededSpecSummary {
            slug: row.try_get("slug")?,
            version: row.try_get("version")?,
            name: row.try_get("name")?,
            rules_hash: row.try_get("rules_hash")?,
        });
    }

    Ok(seeded)
}

fn load_and_validate_spec(path: &str) -> Result<(String, Value, String), DbError> {
    let raw = fs::read_to_string(path)
        .map_err(|err| DbError::InvalidSpec(format!("failed to read {path}: {err}")))?;

    let spec: GameSpec = serde_json::from_str(&raw)?;
    spec.validate_specs()
        .map_err(|err| DbError::InvalidSpec(err.to_string()))?;

    let rules_json: Value = serde_json::from_str(&raw)?;

    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let rules_hash = encode(hasher.finalize());

    Ok((spec.name, rules_json, rules_hash))
}

fn map_row_to_spec(row: sqlx::postgres::PgRow) -> Result<StoredGameSpec, DbError> {
    Ok(StoredGameSpec {
        id: row.try_get("id")?,
        slug: row.try_get("slug")?,
        version: row.try_get("version")?,
        name: row.try_get("name")?,
        rules_json: row.try_get("rules_json")?,
        rules_hash: row.try_get("rules_hash")?,
        is_active: row.try_get("is_active")?,
    })
}
