use chrono::{DateTime, Utc};
use modern_chess_kernel::UserId;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::StoreError;
use crate::{
    DomainError, DraftContent, DraftError, GameStatus, ShareToken, UserGame, UserGameId,
    MAX_DRAFTS_PER_AUTHOR,
};

const COLUMNS: &str =
    "id, author_id, name, rules, skin, status, version, share_token, created_at, updated_at";

/// The games people make. Every call that names a game also names its author, in the query itself,
/// so a game that is not yours is "not found" and there is no way to ask about it.
#[derive(Clone)]
pub struct UserGameRepo {
    pool: PgPool,
}

impl UserGameRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Keeps a new draft, or says the author has too many.
    pub async fn create(&self, game: &UserGame) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await?;
        // One writer per author at a time, so two saves cannot both take the last place.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
            .bind(game.author.to_string())
            .execute(&mut *tx)
            .await?;
        let kept: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM user_games WHERE author_id = $1 AND status = 'DRAFT'",
        )
        .bind(game.author.as_uuid())
        .fetch_one(&mut *tx)
        .await?;
        if kept >= MAX_DRAFTS_PER_AUTHOR {
            return Err(DraftError::TooMany.into());
        }

        sqlx::query(
            "INSERT INTO user_games (id, author_id, name, rules, skin, status, version, share_token, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, NULL, $8, $9)",
        )
        .bind(game.id.as_uuid())
        .bind(game.author.as_uuid())
        .bind(&game.content.name)
        .bind(&game.content.rules)
        .bind(&game.content.skin)
        .bind(game.status.as_db())
        .bind(game.version)
        .bind(game.created_at)
        .bind(game.updated_at)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO user_game_status_history (game_id, from_status, to_status, actor_id, at) \
             VALUES ($1, NULL, $2, $3, $4)",
        )
        .bind(game.id.as_uuid())
        .bind(game.status.as_db())
        .bind(game.author.as_uuid())
        .bind(game.created_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// The author's drafts, the latest changed first.
    pub async fn drafts_of(&self, author: UserId) -> Result<Vec<UserGame>, StoreError> {
        let rows = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM user_games WHERE author_id = $1 AND status = 'DRAFT' \
             ORDER BY updated_at DESC, id"
        ))
        .bind(author.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(game_from_row).collect()
    }

    pub async fn get(
        &self,
        id: UserGameId,
        author: UserId,
    ) -> Result<Option<UserGame>, StoreError> {
        let row = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM user_games WHERE id = $1 AND author_id = $2"
        ))
        .bind(id.as_uuid())
        .bind(author.as_uuid())
        .fetch_optional(&self.pool)
        .await?;
        row.as_ref().map(game_from_row).transpose()
    }

    /// Saves new content over the version the caller started from. A different version, or a game
    /// that is no longer a draft, changes nothing and says why.
    pub async fn update(
        &self,
        id: UserGameId,
        author: UserId,
        based_on: i64,
        content: &DraftContent,
        now: DateTime<Utc>,
    ) -> Result<UserGame, StoreError> {
        let row = sqlx::query(&format!(
            "UPDATE user_games SET name = $4, rules = $5, skin = $6, version = version + 1, updated_at = $7 \
             WHERE id = $1 AND author_id = $2 AND status = 'DRAFT' AND version = $3 \
             RETURNING {COLUMNS}"
        ))
        .bind(id.as_uuid())
        .bind(author.as_uuid())
        .bind(based_on)
        .bind(&content.name)
        .bind(&content.rules)
        .bind(&content.skin)
        .bind(now)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(row) => game_from_row(&row),
            None => Err(self.why_not_changed(id, author).await?),
        }
    }

    /// Deletes a draft for good, with its history. Nothing else is ever deleted by its author.
    pub async fn delete_draft(&self, id: UserGameId, author: UserId) -> Result<(), StoreError> {
        let done = sqlx::query(
            "DELETE FROM user_games WHERE id = $1 AND author_id = $2 AND status = 'DRAFT'",
        )
        .bind(id.as_uuid())
        .bind(author.as_uuid())
        .execute(&self.pool)
        .await?;
        if done.rows_affected() == 1 {
            Ok(())
        } else {
            Err(self.why_not_changed(id, author).await?)
        }
    }

    /// The draft's private link: made the first time, the same one after that.
    pub async fn share(&self, id: UserGameId, author: UserId) -> Result<ShareToken, StoreError> {
        let fresh = ShareToken::generate();
        let token: Option<String> = sqlx::query_scalar(
            "UPDATE user_games SET share_token = COALESCE(share_token, $3) \
             WHERE id = $1 AND author_id = $2 AND status = 'DRAFT' RETURNING share_token",
        )
        .bind(id.as_uuid())
        .bind(author.as_uuid())
        .bind(fresh.as_str())
        .fetch_optional(&self.pool)
        .await?
        .flatten();
        match token {
            Some(raw) => ShareToken::parse(&raw)
                .ok_or_else(|| DomainError::InvalidDraft("stored token".into()).into()),
            None => Err(self.why_not_changed(id, author).await?),
        }
    }

    /// Turns the private link off: it stops working at once. Doing it twice is fine.
    pub async fn unshare(&self, id: UserGameId, author: UserId) -> Result<(), StoreError> {
        let done = sqlx::query("UPDATE user_games SET share_token = NULL WHERE id = $1 AND author_id = $2 AND status = 'DRAFT'")
            .bind(id.as_uuid())
            .bind(author.as_uuid())
            .execute(&self.pool)
            .await?;
        if done.rows_affected() == 1 {
            Ok(())
        } else {
            Err(self.why_not_changed(id, author).await?)
        }
    }

    /// What a private link opens: only the content, and only while the game is a draft.
    pub async fn shared(&self, token: &ShareToken) -> Result<Option<DraftContent>, StoreError> {
        let row = sqlx::query(
            "SELECT name, rules, skin FROM user_games WHERE share_token = $1 AND status = 'DRAFT'",
        )
        .bind(token.as_str())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            Ok(DraftContent {
                name: row.try_get("name")?,
                rules: row.try_get("rules")?,
                skin: row.try_get("skin")?,
            })
        })
        .transpose()
    }

    /// After a change that matched no row: which of the reasons it was.
    async fn why_not_changed(
        &self,
        id: UserGameId,
        author: UserId,
    ) -> Result<StoreError, StoreError> {
        let found = self.get(id, author).await?;
        Ok(match found {
            None => StoreError::NotFound,
            Some(game) if !game.status.is_editable() => DraftError::NotADraft.into(),
            Some(game) => DraftError::Stale {
                current: game.version,
            }
            .into(),
        })
    }
}

fn game_from_row(row: &PgRow) -> Result<UserGame, StoreError> {
    let status: String = row.try_get("status")?;
    let token: Option<String> = row.try_get("share_token")?;
    Ok(UserGame {
        id: UserGameId::from_uuid(row.try_get::<Uuid, _>("id")?),
        author: UserId::from_uuid(row.try_get::<Uuid, _>("author_id")?),
        content: DraftContent {
            name: row.try_get("name")?,
            rules: row.try_get("rules")?,
            skin: row.try_get("skin")?,
        },
        status: GameStatus::from_db(&status)
            .ok_or_else(|| DomainError::InvalidDraft(format!("unknown status {status}")))?,
        version: row.try_get("version")?,
        share_token: token.as_deref().and_then(ShareToken::parse),
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}
