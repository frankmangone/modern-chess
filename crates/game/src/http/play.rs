use std::panic::{catch_unwind, AssertUnwindSafe};

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
    Variant,
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
    let variant = state.variants.get(instance.slug()).await?;
    let you = viewer.and_then(|user| instance.seat_of(user.id).cloned());
    let shown = guard(|| snapshot(&instance, variant.as_ref(), &session, you))?;
    Ok(Json(Shown::Changed(Box::new(shown))))
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
    let found = state
        .instances
        .get(id)
        .await?
        .ok_or_else(|| ApiError::not_found("match not found"))?;
    let spec = load_spec(&state, found.spec_id()).await?;
    let variant = state.variants.get(found.slug()).await?;

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
    if guard(|| session.view().turn)? != seat.as_str() {
        return Err(ApiError::new(StatusCode::FORBIDDEN, "it is not your turn"));
    }

    let now = Utc::now();
    let applied = guard(|| match &request.kind {
        CommandKind::Select { x, y } => session.select(*x, *y),
        CommandKind::Move { x, y } => session.make_move(*x, *y),
        CommandKind::Promote { code } => session.promote(code),
        CommandKind::Drop { code, x, y } => session.drop_piece(code, *x, *y),
        CommandKind::Pass => session.pass(),
    })?;
    if let Err(reason) = applied {
        // The reason may quote the engine, so it goes to the log and never to the player.
        info!(instance = %id.as_uuid(), user = %caller.id.as_uuid(), %reason, "command rejected");
        locked.reject(caller.id, request.kind, reason, now).await?;
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "that is not a legal move",
        ));
    }

    let (status, state) = guard(|| (session.view().status, session.state()))?;
    let outcome = match status {
        Status::Playing => None,
        Status::Draw => Some(Outcome::Draw),
        Status::Won { winner } => Some(Outcome::Won {
            seat: SeatName::new(winner),
        }),
    };
    let updated = locked
        .accept(caller.id, request.kind, state, outcome, now)
        .await?;
    Ok(Json(guard(|| {
        snapshot(&updated, variant.as_ref(), &session, Some(seat))
    })?))
}

/// Runs engine code so that a panic inside it becomes an answer instead of a dropped request.
/// The match is stored as plain state and reloaded on every request, so nothing half-applied
/// survives a fault.
fn guard<T>(run: impl FnOnce() -> T) -> Result<T, ApiError> {
    catch_unwind(AssertUnwindSafe(run)).map_err(|panic| {
        let cause = panic
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| panic.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown cause".to_string());
        ApiError::engine_fault(&cause)
    })
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
    guard(|| {
        Session::resume(
            &spec.rules.to_string(),
            instance.state(),
            instance.revision(),
        )
    })?
    .map_err(ApiError::internal)
}

fn snapshot(
    instance: &GameInstance,
    variant: Option<&Variant>,
    session: &Session,
    you: Option<SeatName>,
) -> Snapshot {
    Snapshot {
        summary: InstanceSummary::new(instance, variant),
        view: session.view(),
        outcome: instance.outcome().cloned(),
        you,
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use axum::response::IntoResponse;

    use super::*;

    async fn answer(error: ApiError) -> (StatusCode, Value) {
        let response = error.into_response();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[test]
    fn code_that_finishes_gives_its_value_back() {
        assert_eq!(guard(|| 7).unwrap(), 7);
    }

    #[tokio::test]
    async fn a_panic_becomes_one_fixed_answer_whatever_its_cause() {
        let expected = (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": "the game engine could not handle this position" }),
        );

        let from_str = guard(|| -> u8 { panic!("index out of bounds") }).unwrap_err();
        let from_string = guard(|| -> u8 { panic!("{}", "turn 9 of 2") }).unwrap_err();
        let from_other = guard(|| -> u8 { std::panic::panic_any(42u8) }).unwrap_err();

        assert_eq!(answer(from_str).await, expected);
        assert_eq!(
            answer(from_string).await,
            expected,
            "the cause is never sent"
        );
        assert_eq!(answer(from_other).await, expected);
    }
}
