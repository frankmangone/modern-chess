//! Moving a person's game through review and onto the hub, in one transaction each.
//!
//! Every move reads the game with `FOR UPDATE`, asks the state machine (`domain/lifecycle.rs`)
//! whether the move is legal for the actor, writes the new status and a line of history, and, when
//! the game goes live, writes the whole published game with it: the rules as a new version in
//! `game_specs`, the page and the variant, and the revision. All of it or none of it.
//!
//! Who is asking, and whether the rules are fit to publish, are decided before this: see
//! `crate::Publisher`. What arrives here has already been read by the engine.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use modern_chess_kernel::UserId;
use serde_json::Value;
use sqlx::{PgConnection, Row};

use super::page::save_game_in;
use super::spec::put_version;
use super::user_game::{game_from_row, COLUMNS};
use super::{StoreError, UserGameRepo};
use crate::{
    slug_base, slug_try, transition, Actor, DomainError, DraftError, Event, GamePage, GameSpec,
    GameStatus, Listing, PageSection, Refusal, Slug, UserGame, UserGameId, Variant,
    MAX_DRAFTS_PER_AUTHOR, SORT_ORDER,
};

/// Who hands a game in: the account, and the name it goes by now.
#[derive(Debug, Clone)]
pub struct Author {
    pub id: UserId,
    pub handle: String,
}

/// The game as the engine has read it: what is written when it goes live.
#[derive(Debug, Clone)]
pub struct Prepared {
    pub name: String,
    pub rules: Value,
    pub skin: Option<Value>,
}

/// One of the author's games that is no longer a draft, as their list of games shows it.
#[derive(Debug, Clone)]
pub struct FiledGame {
    pub id: UserGameId,
    pub name: String,
    pub status: GameStatus,
    pub slug: Option<Slug>,
    pub revisions: i64,
    /// The draft the next revision is being written in, if there is one.
    pub working_copy: Option<UserGameId>,
    pub updated_at: DateTime<Utc>,
}

/// What handing a game in did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Filed {
    /// It is on the hub, at this address.
    Published { slug: Slug },
    /// It waits for staff, as the first game of a new author does.
    InReview,
}

impl UserGameRepo {
    /// An author hands in a draft they have just seen at `version`. A person with a game that
    /// has been approved publishes at once; anyone else's goes to review (decision 9).
    pub async fn file(
        &self,
        id: UserGameId,
        author: &Author,
        version: i64,
        listing: &Listing,
        prepared: &Prepared,
        now: DateTime<Utc>,
    ) -> Result<Filed, StoreError> {
        let mut tx = self.pool.begin().await?;
        // One game of a person at a time, so two first games cannot both be told they are
        // trusted before either has been looked at.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
            .bind(author.id.to_string())
            .execute(&mut *tx)
            .await?;
        let game = lock(&mut tx, id, Some(author.id)).await?;
        // Only a draft is handed in. A game that was taken off the hub comes back by its own move.
        if game.status != GameStatus::Draft {
            return Err(DraftError::NotADraft.into());
        }
        if game.version != version {
            return Err(DraftError::Stale {
                current: game.version,
            }
            .into());
        }

        if let Some(original) = game.revises {
            let slug = republish(&mut tx, &game, original, author, listing, prepared, now).await?;
            tx.commit().await?;
            return Ok(Filed::Published { slug });
        }

        let trusted: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM user_game_revisions r \
             JOIN user_games g ON g.id = r.game_id WHERE g.author_id = $1)",
        )
        .bind(author.id.as_uuid())
        .fetch_one(&mut *tx)
        .await?;
        let event = if trusted {
            Event::Publish
        } else {
            Event::Submit
        };
        let to = transition(game.status, event, Actor::Author)?;

        save_listing(&mut tx, id, listing, &author.handle).await?;
        move_to(&mut tx, &game, to, author.id, None, now).await?;
        let filed = if to == GameStatus::Published {
            let slug =
                make_revision(&mut tx, &game, prepared, listing, &author.handle, now).await?;
            Filed::Published { slug }
        } else {
            Filed::InReview
        };
        tx.commit().await?;
        Ok(filed)
    }

    /// The author takes a game back out of review. It is a draft again, with its listing kept.
    pub async fn withdraw(
        &self,
        id: UserGameId,
        author: UserId,
        now: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await?;
        let game = lock(&mut tx, id, Some(author)).await?;
        let to = transition(game.status, Event::Withdraw, Actor::Author)?;
        move_to(&mut tx, &game, to, author, None, now).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Staff accept a game in review: it goes live, at its address, credited to its author.
    pub async fn approve(
        &self,
        id: UserGameId,
        reviewer: UserId,
        actor: Actor,
        prepared: &Prepared,
        now: DateTime<Utc>,
    ) -> Result<Slug, StoreError> {
        let mut tx = self.pool.begin().await?;
        let game = lock(&mut tx, id, None).await?;
        let to = transition(game.status, Event::Approve, actor)?;
        let handle: Option<String> =
            sqlx::query_scalar("SELECT author_handle FROM user_games WHERE id = $1")
                .bind(id.as_uuid())
                .fetch_one(&mut *tx)
                .await?;
        let listing = game
            .listing
            .clone()
            .ok_or_else(|| sqlx::Error::Protocol("a game in review has no listing".into()))?;
        move_to(&mut tx, &game, to, reviewer, None, now).await?;
        let slug = make_revision(
            &mut tx,
            &game,
            prepared,
            &listing,
            handle.as_deref().unwrap_or("someone"),
            now,
        )
        .await?;
        tx.commit().await?;
        Ok(slug)
    }

    /// Staff send a game in review back to its author, with the reason written in its history.
    pub async fn reject(
        &self,
        id: UserGameId,
        reviewer: UserId,
        actor: Actor,
        reason: &str,
        now: DateTime<Utc>,
    ) -> Result<(), StoreError> {
        let mut tx = self.pool.begin().await?;
        let game = lock(&mut tx, id, None).await?;
        let to = transition(game.status, Event::Reject, actor)?;
        move_to(&mut tx, &game, to, reviewer, Some(reason), now).await?;
        tx.commit().await?;
        Ok(())
    }

    /// The author starts changing a published game: a new draft, a working copy that remembers the
    /// game it revises and starts from what is published. A game has one working copy at a time, so
    /// asking again gives the one already there. The boolean says whether it was just made.
    pub async fn revise(
        &self,
        id: UserGameId,
        author: UserId,
        now: DateTime<Utc>,
    ) -> Result<(UserGame, bool), StoreError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
            .bind(author.to_string())
            .execute(&mut *tx)
            .await?;
        let original = lock(&mut tx, id, Some(author)).await?;
        if original.status != GameStatus::Published {
            return Err(Refusal::NotFromHere.into());
        }
        let existing = sqlx::query(&format!(
            "SELECT {COLUMNS} FROM user_games WHERE revises_game_id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(row) = existing {
            return Ok((game_from_row(&row)?, false));
        }
        let kept: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM user_games WHERE author_id = $1 AND status = 'DRAFT'",
        )
        .bind(author.as_uuid())
        .fetch_one(&mut *tx)
        .await?;
        if kept >= MAX_DRAFTS_PER_AUTHOR {
            return Err(DraftError::TooMany.into());
        }

        let mut copy = UserGame::new_draft(author, original.content.clone(), now);
        copy.listing = original.listing.clone();
        copy.revises = Some(id);
        sqlx::query(
            "INSERT INTO user_games \
                 (id, author_id, name, rules, skin, status, version, share_token, \
                  tagline, description, licence, revises_game_id, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, 'DRAFT', 1, NULL, $6, $7, $8, $9, $10, $10)",
        )
        .bind(copy.id.as_uuid())
        .bind(author.as_uuid())
        .bind(&copy.content.name)
        .bind(&copy.content.rules)
        .bind(&copy.content.skin)
        .bind(copy.listing.as_ref().map(|l| l.tagline.as_str()))
        .bind(copy.listing.as_ref().map(|l| l.description.as_str()))
        .bind(copy.listing.as_ref().map(|l| l.licence.as_str()))
        .bind(id.as_uuid())
        .bind(now)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO user_game_status_history (game_id, from_status, to_status, actor_id, at) \
             VALUES ($1, NULL, 'DRAFT', $2, $3)",
        )
        .bind(copy.id.as_uuid())
        .bind(author.as_uuid())
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok((copy, true))
    }

    /// The author's games that are no longer drafts: waiting for review, or on the hub (or taken
    /// off it), with how many revisions each has and the working copy being made of the next.
    pub async fn filed_by(&self, author: UserId) -> Result<Vec<FiledGame>, StoreError> {
        let rows = sqlx::query(
            "SELECT g.id, g.name, g.status, g.slug, g.updated_at, \
                    (SELECT count(*) FROM user_game_revisions r WHERE r.game_id = g.id) AS revisions, \
                    (SELECT c.id FROM user_games c WHERE c.revises_game_id = g.id) AS working_copy \
             FROM user_games g WHERE g.author_id = $1 AND g.status <> 'DRAFT' \
             ORDER BY g.updated_at DESC, g.id",
        )
        .bind(author.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                let status: String = row.try_get("status")?;
                let slug: Option<String> = row.try_get("slug")?;
                Ok(FiledGame {
                    id: UserGameId::from_uuid(row.try_get("id")?),
                    name: row.try_get("name")?,
                    status: GameStatus::from_db(&status).ok_or_else(|| {
                        DomainError::InvalidDraft(format!("unknown status {status}"))
                    })?,
                    slug: slug.map(|raw| Slug::parse(&raw)).transpose()?,
                    revisions: row.try_get("revisions")?,
                    working_copy: row
                        .try_get::<Option<uuid::Uuid>, _>("working_copy")?
                        .map(UserGameId::from_uuid),
                    updated_at: row.try_get("updated_at")?,
                })
            })
            .collect()
    }

    /// Why staff sent each of the author's drafts back, for those whose last move was that. A draft
    /// that was withdrawn, or handed in again since, has nothing to show.
    pub async fn rejections_for(
        &self,
        author: UserId,
    ) -> Result<HashMap<UserGameId, String>, StoreError> {
        let rows = sqlx::query(
            "SELECT DISTINCT ON (h.game_id) h.game_id, h.from_status, h.to_status, h.reason \
             FROM user_game_status_history h JOIN user_games g ON g.id = h.game_id \
             WHERE g.author_id = $1 ORDER BY h.game_id, h.id DESC",
        )
        .bind(author.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        let mut reasons = HashMap::new();
        for row in &rows {
            let from: Option<String> = row.try_get("from_status")?;
            let to: String = row.try_get("to_status")?;
            let reason: Option<String> = row.try_get("reason")?;
            if let (Some(from), Some(reason)) = (from, reason) {
                if (from.as_str(), to.as_str()) == ("IN_REVIEW", "DRAFT") {
                    reasons.insert(UserGameId::from_uuid(row.try_get("game_id")?), reason);
                }
            }
        }
        Ok(reasons)
    }

    /// A game by id, whoever its author is. For staff, who are checked by the caller.
    pub async fn get_any(&self, id: UserGameId) -> Result<Option<UserGame>, StoreError> {
        let row = sqlx::query(&format!("SELECT {COLUMNS} FROM user_games WHERE id = $1"))
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(game_from_row).transpose()
    }
}

/// The game, held until the end of the transaction. `owner` is given when the caller is the
/// author, and a game that is somebody else's is then "not found".
async fn lock(
    tx: &mut PgConnection,
    id: UserGameId,
    owner: Option<UserId>,
) -> Result<UserGame, StoreError> {
    let row = sqlx::query(&format!(
        "SELECT {COLUMNS} FROM user_games WHERE id = $1 AND ($2::uuid IS NULL OR author_id = $2) FOR UPDATE"
    ))
    .bind(id.as_uuid())
    .bind(owner.map(|o| o.as_uuid()))
    .fetch_optional(&mut *tx)
    .await?;
    row.as_ref()
        .map(game_from_row)
        .transpose()?
        .ok_or(StoreError::NotFound)
}

async fn save_listing(
    tx: &mut PgConnection,
    id: UserGameId,
    listing: &Listing,
    handle: &str,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE user_games SET tagline = $2, description = $3, licence = $4, author_handle = $5 \
         WHERE id = $1",
    )
    .bind(id.as_uuid())
    .bind(&listing.tagline)
    .bind(&listing.description)
    .bind(&listing.licence)
    .bind(handle)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Writes the new status and its line of history. The database trigger is the second opinion on
/// the move; `transition` has already given the first.
async fn move_to(
    tx: &mut PgConnection,
    game: &UserGame,
    to: GameStatus,
    actor: UserId,
    reason: Option<&str>,
    now: DateTime<Utc>,
) -> Result<(), StoreError> {
    sqlx::query(
        "UPDATE user_games SET status = $2, share_token = NULL, updated_at = $3 WHERE id = $1",
    )
    .bind(game.id.as_uuid())
    .bind(to.as_db())
    .bind(now)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO user_game_status_history (game_id, from_status, to_status, actor_id, reason, at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(game.id.as_uuid())
    .bind(game.status.as_db())
    .bind(to.as_db())
    .bind(actor.as_uuid())
    .bind(reason)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    Ok(())
}

/// Makes the game's next revision: its address (the first time), its rules as a new version, its
/// page and variant, and the record of what was published. Returns the address.
async fn make_revision(
    tx: &mut PgConnection,
    game: &UserGame,
    prepared: &Prepared,
    listing: &Listing,
    handle: &str,
    now: DateTime<Utc>,
) -> Result<Slug, StoreError> {
    let slug = match &game.slug {
        Some(slug) => slug.clone(),
        None => take_address(tx, game.id, &prepared.name).await?,
    };

    let spec = put_version(
        tx,
        GameSpec::new(
            slug.clone(),
            1,
            prepared.name.clone(),
            prepared.rules.clone(),
            now,
        ),
    )
    .await?;

    let page = GamePage {
        slug: slug.clone(),
        name: Some(prepared.name.clone()),
        preview_variant: Some(slug.clone()),
        tagline: listing.tagline.clone(),
        description: listing.description.clone(),
        tags: Vec::new(),
        accent: None,
        cover_image: None,
        skin: prepared.skin.clone(),
        theme: None,
        sections: vec![PageSection::Preview, PageSection::OpenLobbies],
        parent: None,
        featured: false,
        sort_order: SORT_ORDER,
    };
    save_game_in(tx, &page, &[Variant::single(&slug, &prepared.name)]).await?;

    let revision: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(revision), 0) + 1 FROM user_game_revisions WHERE game_id = $1",
    )
    .bind(game.id.as_uuid())
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO user_game_revisions \
             (game_id, revision, spec_id, rules_hash, author_handle, name, tagline, description, \
              skin, licence, published_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(game.id.as_uuid())
    .bind(revision)
    .bind(spec.id.as_uuid())
    .bind(&spec.rules_hash)
    .bind(handle)
    .bind(&prepared.name)
    .bind(&listing.tagline)
    .bind(&listing.description)
    .bind(&prepared.skin)
    .bind(&listing.licence)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    Ok(slug)
}

/// The first free address that the game's name asks for, kept for the game for good. An address
/// is free when no game, variant, rules or person's game has it.
async fn take_address(
    tx: &mut PgConnection,
    id: UserGameId,
    name: &str,
) -> Result<Slug, StoreError> {
    // One person at a time takes an address, so two cannot be given the same one. Publishing is
    // rare, and this holds only until the transaction ends.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('user-game-addresses'))")
        .execute(&mut *tx)
        .await?;
    let base = slug_base(name)?;
    let mut n = 1;
    let slug = loop {
        let candidate = slug_try(&base, n)?;
        let taken: bool = sqlx::query(
            "SELECT EXISTS (SELECT 1 FROM game_pages WHERE slug = $1) \
                 OR EXISTS (SELECT 1 FROM game_variants WHERE slug = $1) \
                 OR EXISTS (SELECT 1 FROM game_specs WHERE slug = $1) \
                 OR EXISTS (SELECT 1 FROM user_games WHERE slug = $1) AS taken",
        )
        .bind(candidate.as_str())
        .fetch_one(&mut *tx)
        .await?
        .try_get("taken")?;
        if !taken {
            break candidate;
        }
        n += 1;
    };
    sqlx::query("UPDATE user_games SET slug = $2 WHERE id = $1")
        .bind(id.as_uuid())
        .bind(slug.as_str())
        .execute(&mut *tx)
        .await?;
    Ok(slug)
}

/// A working copy is handed in: its content becomes the next revision of the game it revises, under
/// the same address, and the working copy is deleted. Nothing changes state: the original is
/// published before and after. A game that is no longer published cannot be revised.
async fn republish(
    tx: &mut PgConnection,
    copy: &UserGame,
    original: UserGameId,
    author: &Author,
    listing: &Listing,
    prepared: &Prepared,
    now: DateTime<Utc>,
) -> Result<Slug, StoreError> {
    let game = lock(&mut *tx, original, Some(author.id)).await?;
    if game.status != GameStatus::Published {
        return Err(Refusal::NotFromHere.into());
    }
    // The original holds the latest published content, so a new working copy starts from it. Its
    // revisions are what never change.
    sqlx::query(
        "UPDATE user_games SET name = $2, rules = $3, skin = $4, version = version + 1, \
         tagline = $5, description = $6, licence = $7, author_handle = $8, updated_at = $9 \
         WHERE id = $1",
    )
    .bind(original.as_uuid())
    .bind(&copy.content.name)
    .bind(&copy.content.rules)
    .bind(&copy.content.skin)
    .bind(&listing.tagline)
    .bind(&listing.description)
    .bind(&listing.licence)
    .bind(&author.handle)
    .bind(now)
    .execute(&mut *tx)
    .await?;
    let slug = make_revision(tx, &game, prepared, listing, &author.handle, now).await?;
    sqlx::query("DELETE FROM user_games WHERE id = $1")
        .bind(copy.id.as_uuid())
        .execute(&mut *tx)
        .await?;
    Ok(slug)
}
