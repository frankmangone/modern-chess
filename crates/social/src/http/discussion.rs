//! Discussion on a game's page: threads and posts. Reading is open to everyone; writing needs a
//! signed-in account that has chosen its username. See
//! `docs/superpowers/specs/2026-11-28-discussion-design.md`.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, TimeZone, Utc};
use modern_chess_kernel::CurrentUser;
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

use super::error::ApiError;
use super::SocialState;
use crate::domain::post::{Event, PostStatus};
use crate::domain::text;
use crate::pages::looks_like_a_slug;
use crate::store::{DiscussionError, PostRow, ThreadSummary};
use crate::User;

const THREADS_PER_PAGE: i64 = 20;
const POSTS_PER_PAGE: i64 = 50;

impl From<DiscussionError> for ApiError {
    fn from(error: DiscussionError) -> Self {
        match error {
            DiscussionError::Database(cause) => ApiError::internal(cause),
            DiscussionError::TooSoon => ApiError::TooFast {
                code: "too_soon",
                message: "You are posting too fast. Wait a little and try again.",
            },
            DiscussionError::NoSuchThread | DiscussionError::NoSuchPost => ApiError::NotFound,
            DiscussionError::ThreadClosed => ApiError::Conflict {
                code: "thread_closed",
                message: "This thread is closed.",
            },
            DiscussionError::ThreadFull => ApiError::Conflict {
                code: "thread_full",
                message: "This thread is full.",
            },
            DiscussionError::Refused(_) => ApiError::Conflict {
                code: "not_allowed",
                message: "That cannot be done to this post.",
            },
        }
    }
}

fn rejected(problem: text::TextProblem) -> ApiError {
    ApiError::Rejected {
        code: problem.code(),
        message: problem.message(),
    }
}

/// Fails unless the game's page exists and is open: a discussion goes with its page.
async fn open_page(state: &SocialState, slug: &str) -> Result<(), ApiError> {
    let pages = state
        .pages
        .as_ref()
        .ok_or(ApiError::Unavailable("discussion is not available"))?;
    if !looks_like_a_slug(slug) {
        return Err(ApiError::NotFound);
    }
    match pages.is_open(slug).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(ApiError::NotFound),
        Err(cause) => {
            warn!("discussion could not ask about a game page: {cause}");
            Err(ApiError::Unavailable(
                "Discussion can't be reached right now. Try again in a moment.",
            ))
        }
    }
}

/// The caller as someone who may write: an account (not a guest) that has chosen its username.
async fn writer(state: &SocialState, caller: &CurrentUser) -> Result<User, ApiError> {
    let user = state
        .users
        .get(caller.id)
        .await
        .map_err(ApiError::internal)?
        .ok_or(ApiError::Unauthorized)?;
    if !caller.account {
        return Err(ApiError::Forbidden {
            code: "account_required",
            message: "Sign in to join the discussion.",
        });
    }
    if user.setup_pending {
        return Err(ApiError::Forbidden {
            code: "setup_pending",
            message: "Choose a username first.",
        });
    }
    Ok(user)
}

fn parse_id(text: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(text).map_err(|_| ApiError::NotFound)
}

/// A thread list cursor: when the last thread of the page before was last active, and its id.
fn encode_cursor(at: DateTime<Utc>, id: Uuid) -> String {
    format!("{}.{id}", at.timestamp_micros())
}

fn decode_cursor(text: &str) -> Option<(DateTime<Utc>, Uuid)> {
    let (micros, id) = text.split_once('.')?;
    let at = Utc.timestamp_micros(micros.parse().ok()?).single()?;
    Some((at, Uuid::parse_str(id).ok()?))
}

#[derive(Serialize)]
pub struct ThreadItem {
    id: Uuid,
    title: String,
    author: String,
    created_at: DateTime<Utc>,
    last_activity_at: DateTime<Utc>,
    replies: i64,
}

impl From<ThreadSummary> for ThreadItem {
    fn from(thread: ThreadSummary) -> Self {
        Self {
            id: thread.id,
            title: thread.title,
            author: thread.author,
            created_at: thread.created_at,
            last_activity_at: thread.last_activity_at,
            replies: thread.replies,
        }
    }
}

#[derive(Serialize)]
pub struct ThreadList {
    threads: Vec<ThreadItem>,
    /// Pass as `before` to get the next page; absent on the last.
    next: Option<String>,
}

#[derive(Deserialize)]
pub struct Before {
    before: Option<String>,
}

/// `GET /games/{slug}/threads`: newest activity first, twenty at a time.
pub async fn threads(
    State(state): State<SocialState>,
    Path(slug): Path<String>,
    Query(page): Query<Before>,
) -> Result<Json<ThreadList>, ApiError> {
    open_page(&state, &slug).await?;
    let after = match page.before.as_deref() {
        Some(text) => Some(decode_cursor(text).ok_or(ApiError::Rejected {
            code: "bad_cursor",
            message: "That page is not valid.",
        })?),
        None => None,
    };
    let mut found = state
        .discussion
        .threads(&slug, after, THREADS_PER_PAGE + 1)
        .await?;
    let more = found.len() as i64 > THREADS_PER_PAGE;
    found.truncate(THREADS_PER_PAGE as usize);
    let next = if more {
        found
            .last()
            .map(|last| encode_cursor(last.last_activity_at, last.id))
    } else {
        None
    };
    Ok(Json(ThreadList {
        threads: found.into_iter().map(ThreadItem::from).collect(),
        next,
    }))
}

#[derive(Deserialize)]
pub struct NewThread {
    title: String,
    body: String,
}

#[derive(Serialize)]
pub struct Created {
    id: Uuid,
}

/// `POST /games/{slug}/threads`: starts a thread, its body being the opening post.
pub async fn start(
    State(state): State<SocialState>,
    caller: CurrentUser,
    Path(slug): Path<String>,
    Json(request): Json<NewThread>,
) -> Result<(StatusCode, Json<Created>), ApiError> {
    let user = writer(&state, &caller).await?;
    let title = text::title(&request.title).map_err(rejected)?;
    let body = text::body(&request.body).map_err(rejected)?;
    open_page(&state, &slug).await?;
    let id = state
        .discussion
        .start_thread(&slug, user.id, &title, &body, Utc::now(), state.limits)
        .await?;
    Ok((StatusCode::CREATED, Json(Created { id })))
}

#[derive(Serialize)]
pub struct PostItem {
    id: Uuid,
    author: String,
    opening: bool,
    created_at: DateTime<Utc>,
    status: PostStatus,
    /// The words, only while the post is visible.
    body: Option<String>,
    /// Whether the caller wrote it, so the page can offer "Delete".
    mine: bool,
}

fn post_item(post: PostRow, caller: Option<&CurrentUser>) -> PostItem {
    PostItem {
        mine: caller.is_some_and(|caller| caller.id == post.author_id),
        id: post.id,
        author: post.author,
        opening: post.opening,
        created_at: post.created_at,
        status: post.status,
        body: post.body,
    }
}

#[derive(Serialize)]
pub struct ThreadHeadItem {
    id: Uuid,
    game_slug: String,
    title: String,
    author: String,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct ThreadPage {
    thread: ThreadHeadItem,
    posts: Vec<PostItem>,
    /// Pass as `after` to get the next posts; absent on the last page.
    next: Option<i64>,
}

#[derive(Deserialize)]
pub struct After {
    after: Option<i64>,
}

/// `GET /threads/{id}`: the thread and its posts in the order they were written, fifty at a time.
pub async fn thread(
    State(state): State<SocialState>,
    caller: Option<CurrentUser>,
    Path(id): Path<String>,
    Query(page): Query<After>,
) -> Result<Json<ThreadPage>, ApiError> {
    let id = parse_id(&id)?;
    let head = state
        .discussion
        .thread(id)
        .await?
        .ok_or(ApiError::NotFound)?;
    open_page(&state, &head.game_slug).await?;
    let after = page.after.unwrap_or(0).max(0);
    let mut found = state
        .discussion
        .posts(id, after, POSTS_PER_PAGE + 1)
        .await?;
    let more = found.len() as i64 > POSTS_PER_PAGE;
    found.truncate(POSTS_PER_PAGE as usize);
    let next = if more {
        found.last().map(|last| last.seq)
    } else {
        None
    };
    Ok(Json(ThreadPage {
        thread: ThreadHeadItem {
            id: head.id,
            game_slug: head.game_slug,
            title: head.title,
            author: head.author,
            created_at: head.created_at,
        },
        posts: found
            .into_iter()
            .map(|post| post_item(post, caller.as_ref()))
            .collect(),
        next,
    }))
}

#[derive(Deserialize)]
pub struct NewPost {
    body: String,
}

/// `POST /threads/{id}/posts`: a reply.
pub async fn reply(
    State(state): State<SocialState>,
    caller: CurrentUser,
    Path(id): Path<String>,
    Json(request): Json<NewPost>,
) -> Result<(StatusCode, Json<Created>), ApiError> {
    let user = writer(&state, &caller).await?;
    let body = text::body(&request.body).map_err(rejected)?;
    let id = parse_id(&id)?;
    let head = state
        .discussion
        .thread(id)
        .await?
        .ok_or(ApiError::NotFound)?;
    open_page(&state, &head.game_slug).await?;
    let post = state
        .discussion
        .reply(id, user.id, &body, Utc::now(), state.limits)
        .await?;
    Ok((StatusCode::CREATED, Json(Created { id: post })))
}

/// `DELETE /posts/{id}`: the author deletes their own post. It stays as a grey line.
pub async fn delete_post(
    State(state): State<SocialState>,
    caller: CurrentUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = parse_id(&id)?;
    // Staff are not given a way to delete here: they hide. Passing `false` makes anyone but the
    // author a stranger to the post.
    state
        .discussion
        .move_post(id, Event::Delete, caller.id, false, None, Utc::now())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
