//! What staff do with games waiting to be published: see the queue, read a game, approve it or
//! send it back with a reason. Everything here is for staff, who are the ids in the server's list
//! and nobody else: anyone else is refused with 403, and the page says so.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use modern_chess_kernel::AccountUser;
use serde::{Deserialize, Serialize};

use super::drafts::{not_found, parse_id, refuse, unreadable};
use super::error::ApiError;
use super::GameState;
use crate::store::Waiting;
use crate::{Listing, UserGameId};

/// A game in the queue, as a line in the list: no text of the rules.
#[derive(Serialize)]
pub(super) struct QueueJson {
    id: UserGameId,
    name: String,
    author: String,
    listing: Option<Listing>,
    submitted_at: DateTime<Utc>,
}

/// A game in the queue, in full: everything staff need to read it and play it.
#[derive(Serialize)]
pub(super) struct ReviewJson {
    #[serde(flatten)]
    line: QueueJson,
    rules: String,
    skin: String,
}

impl From<&Waiting> for QueueJson {
    fn from(waiting: &Waiting) -> Self {
        Self {
            id: waiting.game.id,
            name: waiting.game.content.name.clone(),
            author: waiting.author_handle.clone(),
            listing: waiting.game.listing.clone(),
            submitted_at: waiting.submitted_at,
        }
    }
}

/// `GET /review`: the games waiting, the longest first.
pub async fn queue(
    State(state): State<GameState>,
    caller: AccountUser,
) -> Result<Json<Vec<QueueJson>>, ApiError> {
    let waiting = state
        .publisher()
        .waiting(caller.id, None)
        .await
        .map_err(refuse)?;
    Ok(Json(waiting.iter().map(QueueJson::from).collect()))
}

/// `GET /review/{id}`: one waiting game, with its rules and look.
pub async fn show(
    State(state): State<GameState>,
    caller: AccountUser,
    Path(id): Path<String>,
) -> Result<Json<ReviewJson>, ApiError> {
    let id = parse_id(&id)?;
    let waiting = state
        .publisher()
        .waiting(caller.id, Some(id))
        .await
        .map_err(refuse)?;
    let one = waiting.into_iter().next().ok_or_else(not_found)?;
    Ok(Json(ReviewJson {
        line: QueueJson::from(&one),
        rules: one.game.content.rules,
        skin: one.game.content.skin,
    }))
}

/// `POST /review/{id}/approve`: the game goes live. The answer is its address.
pub async fn approve(
    State(state): State<GameState>,
    caller: AccountUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let slug = state
        .publisher()
        .approve(parse_id(&id)?, caller.id, Utc::now())
        .await
        .map_err(refuse)?;
    Ok(Json(serde_json::json!({ "slug": slug.as_str() })))
}

#[derive(Deserialize)]
pub struct Rejection {
    reason: String,
}

/// `POST /review/{id}/reject`: the game goes back to its author, with the reason.
pub async fn reject(
    State(state): State<GameState>,
    caller: AccountUser,
    Path(id): Path<String>,
    body: Result<Json<Rejection>, JsonRejection>,
) -> Result<StatusCode, ApiError> {
    let Json(rejection) = body.map_err(unreadable)?;
    state
        .publisher()
        .reject(parse_id(&id)?, caller.id, &rejection.reason, Utc::now())
        .await
        .map_err(refuse)?;
    Ok(StatusCode::NO_CONTENT)
}
