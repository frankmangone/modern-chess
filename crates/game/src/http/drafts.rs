//! The games people are writing: saved to an account, and shared by a private link. The server keeps
//! and hands back text and never runs it: the engine that judges a draft runs in the browser.
//!
//! Keeping work on the server is for accounts: guests are free to make, so every call here but the
//! public link takes an [`AccountUser`], and a guest's token is refused.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use chrono::{DateTime, Utc};
use modern_chess_kernel::AccountUser;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::ApiError;
use super::GameState;
use crate::store::StoreError;
use crate::{
    Author, DomainError, DraftContent, DraftError, Filed, GameStatus, Listing, Refusal, ShareToken,
    UserGame, UserGameId,
};

/// A draft, as its author sees it.
#[derive(Serialize)]
pub(super) struct DraftJson {
    id: UserGameId,
    name: String,
    rules: String,
    skin: String,
    /// Counts the saves; a save names the one it started from.
    version: i64,
    /// Where the game is in its life: a draft, waiting for review, or published.
    status: GameStatus,
    /// The game's address on the hub, once it has one.
    slug: Option<String>,
    /// What the page says, once the game has been handed in (or kept from the published game that
    /// this draft is a new revision of).
    listing: Option<Listing>,
    /// For a new revision of a published game: that game.
    revises: Option<UserGameId>,
    /// Why staff sent the game back, when that was the last thing that happened to it.
    rejection: Option<String>,
    /// The secret of the private link, when the draft is shared.
    share_token: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<UserGame> for DraftJson {
    fn from(game: UserGame) -> Self {
        Self {
            id: game.id,
            name: game.content.name,
            rules: game.content.rules,
            skin: game.content.skin,
            version: game.version,
            status: game.status,
            slug: game.slug.map(|s| s.as_str().to_string()),
            listing: game.listing,
            revises: game.revises,
            rejection: None,
            share_token: game.share_token.map(|t| t.as_str().to_string()),
            created_at: game.created_at,
            updated_at: game.updated_at,
        }
    }
}

#[derive(Deserialize)]
pub struct NewDraft {
    name: String,
    rules: String,
    #[serde(default)]
    skin: String,
}

#[derive(Deserialize)]
pub struct DraftChange {
    name: String,
    rules: String,
    #[serde(default)]
    skin: String,
    /// The version this change starts from.
    version: i64,
}

pub(super) fn parse_id(raw: &str) -> Result<UserGameId, ApiError> {
    Uuid::parse_str(raw)
        .map(UserGameId::from_uuid)
        .map_err(|_| not_found())
}

pub(super) fn not_found() -> ApiError {
    ApiError::not_found("draft not found")
}

/// A body that could not be read: too big is its own answer, anything else is "could not be read".
pub(super) fn unreadable(rejection: JsonRejection) -> ApiError {
    if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "That is much more than a draft can hold.",
        )
    } else {
        ApiError::bad_request()
    }
}

/// A refusal, said to the person in words, never with the store's own text.
pub(super) fn refuse(err: StoreError) -> ApiError {
    match err {
        StoreError::NotFound => not_found(),
        StoreError::Draft(DraftError::TooMany) => ApiError::new(
            StatusCode::CONFLICT,
            "You have as many drafts as can be kept. Delete one first.",
        ),
        StoreError::Draft(DraftError::NotADraft) => {
            ApiError::new(StatusCode::CONFLICT, "Only a draft can be changed.")
        }
        StoreError::Draft(DraftError::Stale { current }) => ApiError::new(
            StatusCode::CONFLICT,
            "This draft was changed somewhere else.",
        )
        .with_version(current),
        StoreError::Domain(DomainError::InvalidRules(reason)) => ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("These rules cannot be published: {reason}."),
        ),
        StoreError::Refused(Refusal::NotFromHere) => ApiError::new(
            StatusCode::CONFLICT,
            "The game is not in a state where that can be done.",
        ),
        StoreError::Refused(Refusal::NotYours) => {
            ApiError::new(StatusCode::FORBIDDEN, "That is not yours to do.")
        }
        StoreError::Engine(why) => ApiError::engine_fault(&why),
        StoreError::Domain(DomainError::InvalidDraft(reason)) => ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("This draft cannot be kept: {reason}."),
        ),
        other => ApiError::internal(other),
    }
}

fn content(name: String, rules: String, skin: String) -> Result<DraftContent, ApiError> {
    DraftContent::new(name, rules, skin).map_err(|err| refuse(err.into()))
}

/// `GET /drafts`: the caller's drafts.
pub async fn list(
    State(state): State<GameState>,
    caller: AccountUser,
) -> Result<Json<Vec<DraftJson>>, ApiError> {
    let drafts = state
        .user_games
        .drafts_of(caller.id)
        .await
        .map_err(refuse)?;
    let mut why = state
        .user_games
        .rejections_for(caller.id)
        .await
        .map_err(refuse)?;
    Ok(Json(
        drafts
            .into_iter()
            .map(|game| {
                let rejection = why.remove(&game.id);
                DraftJson {
                    rejection,
                    ..DraftJson::from(game)
                }
            })
            .collect(),
    ))
}

/// `POST /drafts`: keeps a new draft.
pub async fn create(
    State(state): State<GameState>,
    caller: AccountUser,
    body: Result<Json<NewDraft>, JsonRejection>,
) -> Result<(StatusCode, Json<DraftJson>), ApiError> {
    let Json(draft) = body.map_err(unreadable)?;
    let game = UserGame::new_draft(
        caller.id,
        content(draft.name, draft.rules, draft.skin)?,
        Utc::now(),
    );
    state.user_games.create(&game).await.map_err(refuse)?;
    Ok((StatusCode::CREATED, Json(game.into())))
}

/// `GET /drafts/{id}`: one of the caller's drafts. Anyone else's is not found.
pub async fn show(
    State(state): State<GameState>,
    caller: AccountUser,
    Path(id): Path<String>,
) -> Result<Json<DraftJson>, ApiError> {
    let game = state
        .user_games
        .get(parse_id(&id)?, caller.id)
        .await
        .map_err(refuse)?;
    let game = game.ok_or_else(not_found)?;
    let rejection = state
        .user_games
        .rejections_for(caller.id)
        .await
        .map_err(refuse)?
        .remove(&game.id);
    Ok(Json(DraftJson {
        rejection,
        ..DraftJson::from(game)
    }))
}

/// `PUT /drafts/{id}`: saves a change, if nobody saved since the version it starts from.
pub async fn change(
    State(state): State<GameState>,
    caller: AccountUser,
    Path(id): Path<String>,
    body: Result<Json<DraftChange>, JsonRejection>,
) -> Result<Json<DraftJson>, ApiError> {
    let id = parse_id(&id)?;
    let Json(change) = body.map_err(unreadable)?;
    let new = content(change.name, change.rules, change.skin)?;
    let saved = state
        .user_games
        .update(id, caller.id, change.version, &new, Utc::now())
        .await
        .map_err(refuse)?;
    Ok(Json(saved.into()))
}

/// `DELETE /drafts/{id}`.
pub async fn delete(
    State(state): State<GameState>,
    caller: AccountUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .user_games
        .delete_draft(parse_id(&id)?, caller.id)
        .await
        .map_err(refuse)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct Shared {
    token: String,
}

/// `PUT /drafts/{id}/share`: the draft's private link, made the first time and the same after.
pub async fn share(
    State(state): State<GameState>,
    caller: AccountUser,
    Path(id): Path<String>,
) -> Result<Json<Shared>, ApiError> {
    let token = state
        .user_games
        .share(parse_id(&id)?, caller.id)
        .await
        .map_err(refuse)?;
    Ok(Json(Shared {
        token: token.as_str().to_string(),
    }))
}

/// `DELETE /drafts/{id}/share`: the link stops working.
pub async fn unshare(
    State(state): State<GameState>,
    caller: AccountUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state
        .user_games
        .unshare(parse_id(&id)?, caller.id)
        .await
        .map_err(refuse)?;
    Ok(StatusCode::NO_CONTENT)
}

/// What anyone with the link may read: the game and nothing about who made it.
#[derive(Serialize)]
struct SharedGame {
    name: String,
    rules: String,
    skin: String,
}

/// `GET /shared/{token}`.
pub async fn open_shared(
    State(state): State<GameState>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let missing = || ApiError::not_found("this link does not open anything");
    let token = ShareToken::parse(&token).ok_or_else(missing)?;
    let game = state
        .user_games
        .shared(&token)
        .await
        .map_err(refuse)?
        .ok_or_else(missing)?;
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(SharedGame {
            name: game.name,
            rules: game.rules,
            skin: game.skin,
        }),
    ))
}

/// What the author sends to publish: the version they saw, and the words of the page.
#[derive(Deserialize)]
pub struct PublishRequest {
    version: i64,
    #[serde(flatten)]
    listing: Listing,
}

/// `POST /drafts/{id}/publish`: hands the draft in. The answer is 200 with the game's address when
/// it is on the hub, or 202 when it waits for staff (the first game of a new author does).
pub async fn publish(
    State(state): State<GameState>,
    caller: AccountUser,
    Path(id): Path<String>,
    body: Result<Json<PublishRequest>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let id = parse_id(&id)?;
    let Json(request) = body.map_err(unreadable)?;
    let author = Author {
        id: caller.id,
        handle: caller.handle.to_string(),
    };
    let filed = state
        .publisher()
        .publish(id, author, request.version, request.listing, Utc::now())
        .await
        .map_err(refuse)?;
    Ok(match filed {
        Filed::Published { slug } => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "published", "slug": slug.as_str() })),
        ),
        Filed::InReview => (
            StatusCode::ACCEPTED,
            Json(serde_json::json!({ "status": "in_review" })),
        ),
    })
}

/// `POST /drafts/{id}/withdraw`: takes a game back out of review.
pub async fn withdraw(
    State(state): State<GameState>,
    caller: AccountUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let id = parse_id(&id)?;
    state
        .publisher()
        .withdraw(id, caller.id, Utc::now())
        .await
        .map_err(refuse)?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /games/{id}/revise`: starts a new revision of one of the caller's published games. The
/// answer is a draft to work in (201 when it was just made, 200 when it was already there); handing
/// it in adds the next revision under the game's own address.
pub async fn revise(
    State(state): State<GameState>,
    caller: AccountUser,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<DraftJson>), ApiError> {
    let (copy, made) = state
        .user_games
        .revise(parse_id(&id)?, caller.id, Utc::now())
        .await
        .map_err(refuse)?;
    let status = if made {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(copy.into())))
}

/// A game of the caller's that is no longer a draft.
#[derive(Serialize)]
pub(super) struct FiledJson {
    id: UserGameId,
    name: String,
    status: GameStatus,
    slug: Option<String>,
    revisions: i64,
    /// The draft the next revision is being written in, if there is one.
    working_copy: Option<UserGameId>,
    updated_at: DateTime<Utc>,
}

/// `GET /my-games`: the caller's games in review, published, or taken off the hub.
pub async fn filed(
    State(state): State<GameState>,
    caller: AccountUser,
) -> Result<Json<Vec<FiledJson>>, ApiError> {
    let games = state.user_games.filed_by(caller.id).await.map_err(refuse)?;
    Ok(Json(
        games
            .into_iter()
            .map(|game| FiledJson {
                id: game.id,
                name: game.name,
                status: game.status,
                slug: game.slug.map(|s| s.as_str().to_string()),
                revisions: game.revisions,
                working_copy: game.working_copy,
                updated_at: game.updated_at,
            })
            .collect(),
    ))
}
