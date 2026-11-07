use serde_json::Value;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::infrastructure::db::{
    models::{GameInstanceRecord, GamePlayerRecord, StoredGameSpec},
    DbError,
};

#[derive(Debug, Clone)]
pub struct GamesRepository {
    pool: PgPool,
}

impl GamesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_instance(
        &self,
        spec: &StoredGameSpec,
        initial_state: Value,
    ) -> Result<GameInstanceRecord, DbError> {
        let row = sqlx::query(
            r#"
            INSERT INTO game_instances (
                id,
                spec_id,
                spec_slug,
                spec_version,
                status,
                state_json,
                revision
            )
            VALUES ($1, $2, $3, $4, 'LOBBY', $5, 0)
            RETURNING id, spec_id, spec_slug, spec_version, status, state_json, revision
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(spec.id)
        .bind(spec.slug.as_str())
        .bind(spec.version)
        .bind(initial_state)
        .fetch_one(&self.pool)
        .await?;

        map_instance_row(row)
    }

    pub async fn get_instance(&self, id: Uuid) -> Result<Option<GameInstanceRecord>, DbError> {
        let row = sqlx::query(
            r#"
            SELECT id, spec_id, spec_slug, spec_version, status, state_json, revision
            FROM game_instances
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_instance_row).transpose()
    }

    pub async fn list_players(&self, game_id: Uuid) -> Result<Vec<GamePlayerRecord>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT seat, user_id
            FROM game_players
            WHERE game_id = $1
            ORDER BY seat ASC
            "#,
        )
        .bind(game_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(map_player_row).collect()
    }

    pub async fn resolve_spec_for_game(
        &self,
        game_id: Uuid,
    ) -> Result<Option<StoredGameSpec>, DbError> {
        let row = sqlx::query(
            r#"
            SELECT gs.id, gs.slug, gs.version, gs.name, gs.rules_json, gs.rules_hash, gs.is_active
            FROM game_instances gi
            INNER JOIN game_specs gs ON gs.id = gi.spec_id
            WHERE gi.id = $1
            LIMIT 1
            "#,
        )
        .bind(game_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(map_spec_row).transpose()
    }

    pub async fn get_instance_for_update(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        id: Uuid,
    ) -> Result<Option<GameInstanceRecord>, DbError> {
        let row = sqlx::query(
            r#"
            SELECT id, spec_id, spec_slug, spec_version, status, state_json, revision
            FROM game_instances
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?;

        row.map(map_instance_row).transpose()
    }

    pub async fn get_spec_by_id_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        spec_id: Uuid,
    ) -> Result<Option<StoredGameSpec>, DbError> {
        let row = sqlx::query(
            r#"
            SELECT id, slug, version, name, rules_json, rules_hash, is_active
            FROM game_specs
            WHERE id = $1
            LIMIT 1
            "#,
        )
        .bind(spec_id)
        .fetch_optional(&mut **tx)
        .await?;

        row.map(map_spec_row).transpose()
    }

    pub async fn list_players_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        game_id: Uuid,
    ) -> Result<Vec<GamePlayerRecord>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT seat, user_id
            FROM game_players
            WHERE game_id = $1
            ORDER BY seat ASC
            "#,
        )
        .bind(game_id)
        .fetch_all(&mut **tx)
        .await?;

        rows.into_iter().map(map_player_row).collect()
    }

    pub async fn find_player_by_seat_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        game_id: Uuid,
        seat: &str,
    ) -> Result<Option<GamePlayerRecord>, DbError> {
        let row = sqlx::query(
            r#"
            SELECT seat, user_id
            FROM game_players
            WHERE game_id = $1 AND seat = $2
            LIMIT 1
            "#,
        )
        .bind(game_id)
        .bind(seat)
        .fetch_optional(&mut **tx)
        .await?;

        row.map(map_player_row).transpose()
    }

    pub async fn find_seat_by_user_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        game_id: Uuid,
        user_id: &str,
    ) -> Result<Option<String>, DbError> {
        let seat = sqlx::query_scalar::<_, String>(
            r#"
            SELECT seat
            FROM game_players
            WHERE game_id = $1 AND user_id = $2
            LIMIT 1
            "#,
        )
        .bind(game_id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(seat)
    }

    pub async fn insert_player_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        game_id: Uuid,
        seat: &str,
        user_id: &str,
    ) -> Result<(), DbError> {
        sqlx::query(
            r#"
            INSERT INTO game_players (game_id, seat, user_id)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(game_id)
        .bind(seat)
        .bind(user_id)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub async fn count_players_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        game_id: Uuid,
    ) -> Result<i64, DbError> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*)
            FROM game_players
            WHERE game_id = $1
            "#,
        )
        .bind(game_id)
        .fetch_one(&mut **tx)
        .await?;

        Ok(count)
    }

    pub async fn update_status_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        game_id: Uuid,
        status: &str,
    ) -> Result<(), DbError> {
        sqlx::query(
            r#"
            UPDATE game_instances
            SET status = $2,
                started_at = CASE WHEN $2 = 'ACTIVE' AND started_at IS NULL THEN now() ELSE started_at END,
                finished_at = CASE WHEN $2 = 'FINISHED' THEN now() ELSE finished_at END
            WHERE id = $1
            "#,
        )
        .bind(game_id)
        .bind(status)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub async fn update_state_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        game_id: Uuid,
        state_json: Value,
        status: &str,
        new_revision: i64,
    ) -> Result<(), DbError> {
        sqlx::query(
            r#"
            UPDATE game_instances
            SET state_json = $2,
                status = $3,
                revision = $4,
                started_at = CASE WHEN $3 = 'ACTIVE' AND started_at IS NULL THEN now() ELSE started_at END,
                finished_at = CASE WHEN $3 = 'FINISHED' THEN now() ELSE finished_at END
            WHERE id = $1
            "#,
        )
        .bind(game_id)
        .bind(state_json)
        .bind(status)
        .bind(new_revision)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub async fn next_command_seq_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        game_id: Uuid,
    ) -> Result<i64, DbError> {
        let seq = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COALESCE(MAX(seq), 0) + 1
            FROM game_commands
            WHERE game_id = $1
            "#,
        )
        .bind(game_id)
        .fetch_one(&mut **tx)
        .await?;

        Ok(seq)
    }

    pub async fn insert_command_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        game_id: Uuid,
        seq: i64,
        actor: &str,
        command_type: &str,
        payload_json: Value,
        accepted: bool,
        error: Option<String>,
    ) -> Result<(), DbError> {
        sqlx::query(
            r#"
            INSERT INTO game_commands (
                game_id,
                seq,
                actor,
                command_type,
                payload_json,
                accepted,
                error
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(game_id)
        .bind(seq)
        .bind(actor)
        .bind(command_type)
        .bind(payload_json)
        .bind(accepted)
        .bind(error)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

fn map_instance_row(row: sqlx::postgres::PgRow) -> Result<GameInstanceRecord, DbError> {
    Ok(GameInstanceRecord {
        id: row.try_get("id")?,
        spec_id: row.try_get("spec_id")?,
        spec_slug: row.try_get("spec_slug")?,
        spec_version: row.try_get("spec_version")?,
        status: row.try_get("status")?,
        state_json: row.try_get("state_json")?,
        revision: row.try_get("revision")?,
    })
}

fn map_player_row(row: sqlx::postgres::PgRow) -> Result<GamePlayerRecord, DbError> {
    Ok(GamePlayerRecord {
        seat: row.try_get("seat")?,
        user_id: row.try_get("user_id")?,
    })
}

fn map_spec_row(row: sqlx::postgres::PgRow) -> Result<StoredGameSpec, DbError> {
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
