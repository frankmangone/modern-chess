use std::time::Duration;

use chrono::{DateTime, Utc};
use modern_chess_kernel::UserId;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use thiserror::Error;
use uuid::Uuid;

use crate::domain::post::{transition, Actor, Event, PostStatus, Refusal};

/// A thread takes up to this many posts, the opening one included.
pub const MAX_POSTS_PER_THREAD: i64 = 1000;

/// How soon a person may write again. Checked against their latest post's time in the database, so
/// there is no counter to keep.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Between one thread started and the next.
    pub thread_cooldown: Duration,
    /// Between one post (a thread's opening one counts) and the next.
    pub post_cooldown: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            thread_cooldown: Duration::from_secs(60),
            post_cooldown: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, Error)]
pub enum DiscussionError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    /// The person wrote too recently.
    #[error("too soon after the last post")]
    TooSoon,
    #[error("no such thread")]
    NoSuchThread,
    /// The thread's opening post is hidden or deleted, so nobody adds to it.
    #[error("the thread is closed")]
    ThreadClosed,
    #[error("the thread is full")]
    ThreadFull,
    #[error("no such post")]
    NoSuchPost,
    /// The post is not in a state to make that move, or it is not the caller's to make.
    #[error("{0}")]
    Refused(Refusal),
}

/// A thread as the list shows it.
#[derive(Debug, Clone)]
pub struct ThreadSummary {
    pub id: Uuid,
    pub title: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    /// Replies still visible, not counting the opening post.
    pub replies: i64,
}

/// A thread's own page: what is known before its posts.
#[derive(Debug, Clone)]
pub struct ThreadHead {
    pub id: Uuid,
    pub game_slug: String,
    pub title: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
}

/// A post. The words are kept only while it is visible: any other post is a grey line, and its
/// words leave the database for nobody.
#[derive(Debug, Clone)]
pub struct PostRow {
    pub id: Uuid,
    pub seq: i64,
    pub author_id: UserId,
    pub author: String,
    pub opening: bool,
    pub created_at: DateTime<Utc>,
    pub status: PostStatus,
    pub body: Option<String>,
}

#[derive(Clone)]
pub struct DiscussionRepo {
    pool: PgPool,
}

/// Serialises one person's writing, so two requests at once cannot both pass the rate limit.
async fn lock_writer(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user: UserId,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('discussion:' || $1::text, 0))")
        .bind(user.as_uuid())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

fn since(then: Option<DateTime<Utc>>, now: DateTime<Utc>, cooldown: Duration) -> bool {
    match (then, chrono::Duration::from_std(cooldown)) {
        (Some(then), Ok(cooldown)) => now - then < cooldown,
        _ => false,
    }
}

impl DiscussionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Starts a thread on a game's page, with its opening post. Returns the thread's id.
    pub async fn start_thread(
        &self,
        game_slug: &str,
        author: UserId,
        title: &str,
        body: &str,
        now: DateTime<Utc>,
        limits: Limits,
    ) -> Result<Uuid, DiscussionError> {
        let mut tx = self.pool.begin().await?;
        lock_writer(&mut tx, author).await?;
        let last_thread: Option<DateTime<Utc>> =
            sqlx::query_scalar("SELECT max(created_at) FROM threads WHERE author_id = $1")
                .bind(author.as_uuid())
                .fetch_one(&mut *tx)
                .await?;
        let last_post: Option<DateTime<Utc>> =
            sqlx::query_scalar("SELECT max(created_at) FROM posts WHERE author_id = $1")
                .bind(author.as_uuid())
                .fetch_one(&mut *tx)
                .await?;
        if since(last_thread, now, limits.thread_cooldown)
            || since(last_post, now, limits.post_cooldown)
        {
            return Err(DiscussionError::TooSoon);
        }

        let thread = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO threads (id, game_slug, author_id, title, created_at, last_activity_at) \
             VALUES ($1, $2, $3, $4, $5, $5)",
        )
        .bind(thread)
        .bind(game_slug)
        .bind(author.as_uuid())
        .bind(title)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        insert_post(&mut tx, thread, author, true, body, now).await?;
        tx.commit().await?;
        Ok(thread)
    }

    /// Adds a reply. Returns the post's id.
    pub async fn reply(
        &self,
        thread: Uuid,
        author: UserId,
        body: &str,
        now: DateTime<Utc>,
        limits: Limits,
    ) -> Result<Uuid, DiscussionError> {
        let mut tx = self.pool.begin().await?;
        lock_writer(&mut tx, author).await?;
        let opening: Option<String> = sqlx::query_scalar(
            "SELECT p.status FROM threads t \
             JOIN posts p ON p.thread_id = t.id AND p.opening \
             WHERE t.id = $1 FOR UPDATE OF t",
        )
        .bind(thread)
        .fetch_optional(&mut *tx)
        .await?;
        match opening.as_deref().and_then(PostStatus::from_db) {
            None => return Err(DiscussionError::NoSuchThread),
            Some(PostStatus::Visible) => {}
            Some(_) => return Err(DiscussionError::ThreadClosed),
        }
        let last_post: Option<DateTime<Utc>> =
            sqlx::query_scalar("SELECT max(created_at) FROM posts WHERE author_id = $1")
                .bind(author.as_uuid())
                .fetch_one(&mut *tx)
                .await?;
        if since(last_post, now, limits.post_cooldown) {
            return Err(DiscussionError::TooSoon);
        }
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM posts WHERE thread_id = $1")
            .bind(thread)
            .fetch_one(&mut *tx)
            .await?;
        if count >= MAX_POSTS_PER_THREAD {
            return Err(DiscussionError::ThreadFull);
        }
        let post = insert_post(&mut tx, thread, author, false, body, now).await?;
        sqlx::query(
            "UPDATE threads SET last_activity_at = GREATEST(last_activity_at, $2) WHERE id = $1",
        )
        .bind(thread)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(post)
    }

    /// The threads of a game's page, newest activity first, `limit` of them after the cursor (the
    /// `last_activity_at` and id of the last thread of the page before). A thread whose opening post
    /// is not visible is left out.
    pub async fn threads(
        &self,
        game_slug: &str,
        after: Option<(DateTime<Utc>, Uuid)>,
        limit: i64,
    ) -> Result<Vec<ThreadSummary>, DiscussionError> {
        let rows = sqlx::query(
            "SELECT t.id, t.title, u.handle, t.created_at, t.last_activity_at, \
                    (SELECT count(*) FROM posts r \
                      WHERE r.thread_id = t.id AND NOT r.opening AND r.status = 'VISIBLE') AS replies \
             FROM threads t \
             JOIN users u ON u.id = t.author_id \
             JOIN posts o ON o.thread_id = t.id AND o.opening AND o.status = 'VISIBLE' \
             WHERE t.game_slug = $1 \
               AND ($2::timestamptz IS NULL OR (t.last_activity_at, t.id) < ($2, $3::uuid)) \
             ORDER BY t.last_activity_at DESC, t.id DESC \
             LIMIT $4",
        )
        .bind(game_slug)
        .bind(after.map(|(at, _)| at))
        .bind(after.map(|(_, id)| id))
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.iter()
            .map(|row| {
                Ok(ThreadSummary {
                    id: row.try_get("id")?,
                    title: row.try_get("title")?,
                    author: row.try_get("handle")?,
                    created_at: row.try_get("created_at")?,
                    last_activity_at: row.try_get("last_activity_at")?,
                    replies: row.try_get("replies")?,
                })
            })
            .collect::<Result<_, sqlx::Error>>()
            .map_err(Into::into)
    }

    pub async fn thread(&self, id: Uuid) -> Result<Option<ThreadHead>, DiscussionError> {
        let row = sqlx::query(
            "SELECT t.id, t.game_slug, t.title, u.handle, t.created_at \
             FROM threads t JOIN users u ON u.id = t.author_id WHERE t.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            Ok(ThreadHead {
                id: row.try_get("id")?,
                game_slug: row.try_get("game_slug")?,
                title: row.try_get("title")?,
                author: row.try_get("handle")?,
                created_at: row.try_get("created_at")?,
            })
        })
        .transpose()
        .map_err(|err: sqlx::Error| err.into())
    }

    /// The posts of a thread in the order they were written, `limit` of them after `after_seq`.
    pub async fn posts(
        &self,
        thread: Uuid,
        after_seq: i64,
        limit: i64,
    ) -> Result<Vec<PostRow>, DiscussionError> {
        let rows = sqlx::query(
            "SELECT p.id, p.seq, p.author_id, u.handle, p.opening, p.created_at, p.status, \
                    CASE WHEN p.status = 'VISIBLE' THEN p.body END AS body \
             FROM posts p JOIN users u ON u.id = p.author_id \
             WHERE p.thread_id = $1 AND p.seq > $2 \
             ORDER BY p.seq LIMIT $3",
        )
        .bind(thread)
        .bind(after_seq)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.iter().map(post_from_row).collect()
    }

    /// Moves a post to another status, by the state machine, and writes the history in the same
    /// transaction. `staff` says whether the caller is staff; a caller who is neither the post's
    /// author nor staff is told there is no such post. Returns the new status.
    pub async fn move_post(
        &self,
        post: Uuid,
        event: Event,
        user: UserId,
        staff: bool,
        reason: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<PostStatus, DiscussionError> {
        let mut tx = self.pool.begin().await?;
        let row = sqlx::query("SELECT author_id, status FROM posts WHERE id = $1 FOR UPDATE")
            .bind(post)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(DiscussionError::NoSuchPost)?;
        let author: Uuid = row.try_get("author_id")?;
        let from = PostStatus::from_db(&row.try_get::<String, _>("status")?)
            .ok_or(DiscussionError::NoSuchPost)?;
        let actor = if user.as_uuid() == author {
            Actor::Author
        } else if staff {
            Actor::Staff
        } else {
            return Err(DiscussionError::NoSuchPost);
        };
        let to = transition(from, event, actor).map_err(DiscussionError::Refused)?;
        sqlx::query("UPDATE posts SET status = $2 WHERE id = $1")
            .bind(post)
            .bind(to.as_db())
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO post_status_history (post_id, from_status, to_status, actor, by_user, reason, at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(post)
        .bind(from.as_db())
        .bind(to.as_db())
        .bind(actor.as_db())
        .bind(user.as_uuid())
        .bind(reason)
        .bind(now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(to)
    }
}

async fn insert_post(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    thread: Uuid,
    author: UserId,
    opening: bool,
    body: &str,
    now: DateTime<Utc>,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO posts (id, thread_id, author_id, opening, body, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(thread)
    .bind(author.as_uuid())
    .bind(opening)
    .bind(body)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(id)
}

fn post_from_row(row: &PgRow) -> Result<PostRow, DiscussionError> {
    let status: String = row.try_get("status")?;
    Ok(PostRow {
        id: row.try_get("id")?,
        seq: row.try_get("seq")?,
        author_id: UserId::from_uuid(row.try_get::<Uuid, _>("author_id")?),
        author: row.try_get("handle")?,
        opening: row.try_get("opening")?,
        created_at: row.try_get("created_at")?,
        status: PostStatus::from_db(&status).ok_or(DiscussionError::NoSuchPost)?,
        body: row.try_get("body")?,
    })
}
