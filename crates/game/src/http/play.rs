use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use modern_chess_kernel::CurrentUser;
use modern_chess_session::{Session, Status, View};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;

use super::error::ApiError;
use super::lobbies::{parse_id, InstanceSummary};
use super::GameState;
use crate::{
    CommandKind, GameInstance, GameSpec, InstanceStatus, Outcome, PlayError, SeatName, SpecId,
};

/// A match as a player sees it: who is seated, whose turn it is, and the board to draw.
#[derive(Serialize)]
pub struct Snapshot {
    #[serde(flatten)]
    summary: InstanceSummary,
    view: View,
    outcome: Option<Outcome>,
    /// The caller's seat, or null for a spectator or someone not signed in.
    you: Option<SeatName>,
}

#[derive(Deserialize)]
pub struct Since {
    since: Option<u64>,
}

#[derive(Deserialize)]
pub struct CommandRequest {
    /// The revision the player was looking at. Anything else means they are out of date.
    revision: u64,
    kind: CommandKind,
}

/// Either the full snapshot, or a short note that nothing happened since the caller's copy.
#[derive(Serialize)]
#[serde(untagged)]
pub enum Shown {
    Unchanged(Value),
    Changed(Box<Snapshot>),
}

/// `GET /instances/{id}?since=N`: the match, or `{"changed": false}` when the caller already
/// has revision N. Anyone may look; a valid token only adds the caller's seat.
pub async fn show(
    State(state): State<GameState>,
    viewer: Option<CurrentUser>,
    Path(id): Path<String>,
    since: Result<Query<Since>, QueryRejection>,
) -> Result<Json<Shown>, ApiError> {
    let id = parse_id(&id)?;
    let Query(Since { since }) = since.map_err(|_| ApiError::bad_request())?;
    let instance = state
        .instances
        .get(id)
        .await?
        .ok_or_else(|| ApiError::not_found("match not found"))?;

    if since == Some(instance.revision()) {
        return Ok(Json(Shown::Unchanged(json!({ "changed": false }))));
    }
    let session = open_session(&state, &instance).await?;
    let you = viewer.and_then(|user| instance.seat_of(user.id).cloned());
    Ok(Json(Shown::Changed(Box::new(snapshot(
        &instance, &session, you,
    )))))
}

/// `POST /instances/{id}/commands`: a seated player asks the game to do something.
///
/// The match is locked while it is decided, so two requests for the same position are
/// answered one after the other and only the first can apply.
pub async fn command(
    State(state): State<GameState>,
    caller: CurrentUser,
    Path(id): Path<String>,
    body: Result<Json<CommandRequest>, JsonRejection>,
) -> Result<Json<Snapshot>, ApiError> {
    let id = parse_id(&id)?;
    let Json(request) = body.map_err(|_| ApiError::bad_request())?;

    // The rules are read before the lock: once the match is locked this request holds a
    // connection, and asking the pool for a second one could wait on other such requests.
    let rules = state
        .instances
        .get(id)
        .await?
        .ok_or_else(|| ApiError::not_found("match not found"))?
        .spec_id();
    let spec = load_spec(&state, rules).await?;

    let locked = state.instances.lock(id).await?;
    let instance = locked.instance();
    let seat = instance
        .seat_of(caller.id)
        .cloned()
        .ok_or_else(|| ApiError::new(StatusCode::FORBIDDEN, "you are not playing in this match"))?;
    match instance.status() {
        InstanceStatus::Lobby => return Err(PlayError::NotStarted.into()),
        InstanceStatus::Finished => return Err(PlayError::Over.into()),
        InstanceStatus::Active => {}
    }
    if instance.revision() != request.revision {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "the position has changed",
        ));
    }
    let mut session = resume(&spec, instance)?;
    if session.view().turn != seat.as_str() {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "it is not your turn"));
    }

    let now = Utc::now();
    let applied = match &request.kind {
        CommandKind::Select { x, y } => session.select(*x, *y),
        CommandKind::Move { x, y } => session.make_move(*x, *y),
        CommandKind::Promote { code } => session.promote(code),
        CommandKind::Drop { .. } => Err("drops are not supported yet".to_string()),
    };
    if let Err(reason) = applied {
        // The reason may quote the engine, so it goes to the log and never to the player.
        info!(instance = %id.as_uuid(), user = %caller.id.as_uuid(), %reason, "command rejected");
        locked.reject(caller.id, request.kind, reason, now).await?;
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "that is not a legal move",
        ));
    }

    let outcome = match session.view().status {
        Status::Playing => None,
        Status::Draw => Some(Outcome::Draw),
        Status::Won { winner } => Some(Outcome::Won {
            seat: SeatName::new(winner),
        }),
    };
    let updated = locked
        .accept(caller.id, request.kind, session.state(), outcome, now)
        .await?;
    Ok(Json(snapshot(&updated, &session, Some(seat))))
}

/// Replays a stored match into a session, using the exact rules version it was started with.
async fn open_session(state: &GameState, instance: &GameInstance) -> Result<Session, ApiError> {
    let spec = load_spec(state, instance.spec_id()).await?;
    resume(&spec, instance)
}

async fn load_spec(state: &GameState, id: SpecId) -> Result<GameSpec, ApiError> {
    state
        .specs
        .get(id)
        .await?
        .ok_or_else(|| ApiError::internal("a match refers to a spec that is not stored"))
}

fn resume(spec: &GameSpec, instance: &GameInstance) -> Result<Session, ApiError> {
    Session::resume(
        &spec.rules.to_string(),
        instance.state(),
        instance.revision(),
    )
    .map_err(ApiError::internal)
}

fn snapshot(instance: &GameInstance, session: &Session, you: Option<SeatName>) -> Snapshot {
    Snapshot {
        summary: InstanceSummary::from(instance),
        view: session.view(),
        outcome: instance.outcome().cloned(),
        you,
    }
}
