use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use modern_chess_kernel::CurrentUser;
use modern_chess_session::Choice;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::info;

use super::error::ApiError;
use super::lobbies::{parse_id, InstanceSummary};
use super::GameState;
use crate::{
    CommandKind, GameInstance, GameSpec, InstanceStatus, Move, Outcome, PlayError, SeatName,
    Seeing, SpecId, Standing, Step, Variant, WinReason,
};

/// A match as a player sees it: who is seated, whose turn it is, and the board to draw.
#[derive(Serialize)]
pub struct Snapshot {
    #[serde(flatten)]
    summary: InstanceSummary,
    /// The board as the engine drew it for this viewer, passed through as it came.
    view: Value,
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
    let spec = load_spec(&state, instance.spec_id()).await?;
    let variant = state.variants.get(instance.slug()).await?;
    let you = viewer.and_then(|user| instance.seat_of(user.id).cloned());
    let view = state
        .engine
        .view(
            &spec,
            instance.state(),
            instance.revision(),
            seeing(&instance, &you),
        )
        .await?;
    Ok(Json(Shown::Changed(Box::new(snapshot(
        &instance,
        variant.as_ref(),
        view,
        you,
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
    // Like a body that does not parse, this is refused before anything is logged.
    if let Some(refusal) = request.kind.refusal() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, refusal));
    }

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
    // Giving up is the player's decision whatever the board looks like now, so it is the one
    // command that does not need to be about the latest position, or to be the player's turn.
    let resigning = request.kind == CommandKind::Resign;
    if !resigning && instance.revision() != request.revision {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "the position has changed",
        ));
    }
    if resigning {
        // Only a match between two sides has an opponent to hand the win to.
        let winner = match instance.seats() {
            [first, second] => if *first == seat { second } else { first }.clone(),
            _ => {
                return Err(ApiError::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "resigning is not possible in this game",
                ))
            }
        };
        let outcome = Outcome::Won {
            seat: winner,
            reason: Some(WinReason::Resigned),
        };
        let kept = instance.state().clone();
        let revision = instance.revision();
        // The match is over from here, so it is shown to the one who gave up as to everyone.
        let view = state
            .engine
            .view(&spec, &kept, revision, Seeing::Everyone)
            .await?;
        let updated = locked
            .accept(caller.id, request.kind, kept, Some(outcome), Utc::now())
            .await?;
        return Ok(Json(snapshot(&updated, variant.as_ref(), view, Some(seat))));
    }

    let step = match &request.kind {
        CommandKind::Choose {
            place,
            code,
            held,
            action,
        } => Step::Choose {
            place: place.clone(),
            code: code.clone(),
            held: held.clone(),
            action: action.clone(),
        },
        CommandKind::Cancel => Step::Cancel,
        // Answered above, before the turn is checked.
        CommandKind::Resign => {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "resign is not a move",
            ))
        }
    };
    let now = Utc::now();
    let moved = state
        .engine
        .play(&spec, instance.state(), instance.revision(), &seat, step)
        .await?;
    let (state_after, standing, view) = match moved {
        // The player to move, or a seat that owes an answer to an ask that asks everyone at once.
        Move::NotYourTurn => {
            return Err(ApiError::new(StatusCode::FORBIDDEN, "it is not your turn"))
        }
        Move::Rejected(reason) => {
            // The reason may quote the engine, so it goes to the log and never to the player.
            info!(instance = %id.as_uuid(), user = %caller.id.as_uuid(), %reason, "command rejected");
            locked.reject(caller.id, request.kind, reason, now).await?;
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "that is not a legal move",
            ));
        }
        Move::Played {
            state,
            standing,
            view,
        } => (state, standing, view),
    };

    let outcome = match standing {
        Standing::Playing => None,
        Standing::Draw => Some(Outcome::Draw),
        Standing::Won { winner } => Some(Outcome::Won {
            seat: SeatName::new(winner),
            reason: None,
        }),
    };
    let updated = locked
        .accept(caller.id, request.kind, state_after, outcome, now)
        .await?;
    Ok(Json(snapshot(&updated, variant.as_ref(), view, Some(seat))))
}

async fn load_spec(state: &GameState, id: SpecId) -> Result<GameSpec, ApiError> {
    state
        .specs
        .get(id)
        .await?
        .ok_or_else(|| ApiError::internal("a match refers to a spec that is not stored"))
}

/// Who is looking at a match: a finished one shows everything to everyone, as the engine does
/// for a game that is over.
fn seeing(instance: &GameInstance, you: &Option<SeatName>) -> Seeing {
    match (you, instance.outcome().is_some()) {
        (_, true) => Seeing::Everyone,
        (Some(seat), false) => Seeing::Player {
            seat: seat.as_str().to_string(),
        },
        (None, false) => Seeing::Spectator,
    }
}

/// What `you` is sent: the game as the engine drew it for their seat (a spectator's and a
/// stranger's alike when `you` is none).
fn snapshot(
    instance: &GameInstance,
    variant: Option<&Variant>,
    mut view: Value,
    you: Option<SeatName>,
) -> Snapshot {
    let over = instance.outcome().is_some();
    // What the player to move is weighing is theirs alone, and there is nothing left to weigh
    // once the match is over. The engine already gives it to the player to move and nobody else;
    // a hot-seat viewer or a finished match gets it blanked here.
    let turn = view.get("turn").and_then(Value::as_str);
    let to_move = you.as_ref().is_some_and(|seat| Some(seat.as_str()) == turn);
    if over || !to_move {
        if let Some(choice) = view.get_mut("choice") {
            *choice = serde_json::to_value(Choice::default()).unwrap_or(Value::Null);
        }
    }
    Snapshot {
        summary: InstanceSummary::new(instance, variant),
        view,
        outcome: instance.outcome().cloned(),
        you,
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use axum::response::IntoResponse;

    use super::*;
    use crate::EngineError;

    async fn answer(error: ApiError) -> (StatusCode, Value) {
        let response = error.into_response();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn an_engine_that_stopped_becomes_one_fixed_answer_whatever_its_cause() {
        let expected = (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": "the game engine could not handle this position" }),
        );

        for cause in [
            "index out of bounds",
            "the rules ran out of their time budget",
        ] {
            let error = ApiError::from(EngineError::Stopped(cause.to_string()));
            assert_eq!(answer(error).await, expected, "the cause is never sent");
        }
    }
}
