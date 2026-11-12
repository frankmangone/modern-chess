use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use modern_chess_kernel::CurrentUser;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::catalog::{find_variant, LobbyPlayer};
use super::error::ApiError;
use super::GameState;
use crate::{GameInstance, InstanceId, InstanceStatus, SeatName, Slug, Variant};

/// What anyone with a match's id may see of it.
#[derive(Serialize)]
pub(super) struct InstanceSummary {
    id: InstanceId,
    /// The variant being played: the slug of its rules.
    slug: Slug,
    /// The game that variant belongs to. Its own slug when the variant is not on record.
    game: Slug,
    variant_label: Option<String>,
    status: InstanceStatus,
    revision: u64,
    capacity: usize,
    seats: Vec<SeatName>,
    players: Vec<LobbyPlayer>,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
}

impl InstanceSummary {
    pub(super) fn new(instance: &GameInstance, variant: Option<&Variant>) -> Self {
        Self {
            id: instance.id(),
            slug: instance.slug().clone(),
            game: variant.map_or_else(|| instance.slug().clone(), |v| v.game.clone()),
            variant_label: variant.map(|v| v.label.clone()),
            status: instance.status(),
            revision: instance.revision(),
            capacity: instance.capacity(),
            seats: instance.seats().to_vec(),
            players: instance.players().iter().map(LobbyPlayer::from).collect(),
            created_at: instance.created_at(),
            started_at: instance.started_at(),
        }
    }
}

#[derive(Serialize)]
pub struct Created {
    id: InstanceId,
}

#[derive(Deserialize)]
pub struct JoinRequest {
    seat: SeatName,
}

/// `POST /catalog/{variant}/lobbies`: opens a lobby in a variant and seats the caller in the
/// first seat.
pub async fn create(
    State(state): State<GameState>,
    caller: CurrentUser,
    Path(variant): Path<String>,
) -> Result<(StatusCode, Json<Created>), ApiError> {
    let not_found = || ApiError::not_found("game not found");
    let variant = Slug::parse(&variant).map_err(|_| not_found())?;
    let game = find_variant(&state, &variant)
        .await?
        .ok_or_else(not_found)?;

    let now = Utc::now();
    let mut lobby = GameInstance::new_lobby(&game.spec, now).map_err(ApiError::internal)?;
    let seat = lobby
        .first_free_seat()
        .cloned()
        .ok_or_else(|| ApiError::internal("a new lobby has no free seat"))?;
    lobby
        .join(caller.id, caller.handle, seat, now)
        .map_err(ApiError::internal)?;
    state.instances.create(&lobby).await?;

    Ok((StatusCode::CREATED, Json(Created { id: lobby.id() })))
}

/// `POST /instances/{id}/join`: takes the named seat for the caller.
pub async fn join(
    State(state): State<GameState>,
    caller: CurrentUser,
    Path(id): Path<String>,
    body: Result<Json<JoinRequest>, JsonRejection>,
) -> Result<Json<InstanceSummary>, ApiError> {
    let id = parse_id(&id)?;
    let Json(request) = body.map_err(|_| ApiError::bad_request())?;

    let joined = state
        .instances
        .join(id, caller.id, caller.handle, request.seat, Utc::now())
        .await?;

    let variant = state.variants.get(joined.slug()).await?;
    Ok(Json(InstanceSummary::new(&joined, variant.as_ref())))
}

pub(super) fn parse_id(raw: &str) -> Result<InstanceId, ApiError> {
    Uuid::parse_str(raw)
        .map(InstanceId::from_uuid)
        .map_err(|_| ApiError::not_found("match not found"))
}
