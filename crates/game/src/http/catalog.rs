use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use tracing::warn;

use super::error::ApiError;
use super::GameState;
use crate::store::StoreError;
use crate::{GamePage, GameSpec, InstanceId, SeatName, Slug};

/// One card on the front page.
#[derive(Serialize)]
pub struct CatalogEntry {
    slug: Slug,
    name: String,
    tagline: String,
    tags: Vec<String>,
    accent: Option<String>,
    cover_image: Option<String>,
    featured: bool,
}

/// What the game page needs: its tunable page, the rules to run a preview with, and the
/// lobbies waiting for players.
#[derive(Serialize)]
pub struct GameDetail {
    page: GamePage,
    spec: SpecSummary,
    open_lobbies: Vec<LobbySummary>,
}

#[derive(Serialize)]
pub struct SpecSummary {
    name: String,
    version: u32,
    seats: Vec<SeatName>,
    rules: Value,
}

#[derive(Serialize)]
pub struct LobbySummary {
    id: InstanceId,
    capacity: usize,
    players: Vec<LobbyPlayer>,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
pub struct LobbyPlayer {
    seat: SeatName,
    handle: String,
}

/// A game that can be shown: it has a page, and rules the engine can still read.
struct Playable {
    page: GamePage,
    spec: GameSpec,
    seats: Vec<SeatName>,
}

pub async fn list(State(state): State<GameState>) -> Result<Json<Vec<CatalogEntry>>, ApiError> {
    let mut games = Vec::new();
    for page in state.pages.list().await? {
        if let Some(game) = playable(&state, page).await? {
            games.push(game);
        }
    }
    games.sort_by(|a, b| (a.page.sort_order, &a.spec.name).cmp(&(b.page.sort_order, &b.spec.name)));

    Ok(Json(
        games
            .into_iter()
            .map(|game| CatalogEntry {
                slug: game.page.slug,
                name: game.spec.name,
                tagline: game.page.tagline,
                tags: game.page.tags,
                accent: game.page.accent,
                cover_image: game.page.cover_image,
                featured: game.page.featured,
            })
            .collect(),
    ))
}

pub async fn detail(
    State(state): State<GameState>,
    Path(slug): Path<String>,
) -> Result<Json<GameDetail>, ApiError> {
    let not_found = || ApiError::not_found("game not found");
    let slug = Slug::parse(&slug).map_err(|_| not_found())?;
    let page = state.pages.get(&slug).await?.ok_or_else(not_found)?;
    let game = playable(&state, page).await?.ok_or_else(not_found)?;

    Ok(Json(GameDetail {
        page: game.page,
        spec: SpecSummary {
            name: game.spec.name,
            version: game.spec.version,
            seats: game.seats,
            rules: game.spec.rules,
        },
        open_lobbies: Vec::new(),
    }))
}

/// Pairs a page with the newest rules of its game. A game whose rules are missing, or that
/// the engine can no longer read, is left out and logged, so one bad game never takes the
/// whole catalogue down.
async fn playable(state: &GameState, page: GamePage) -> Result<Option<Playable>, StoreError> {
    let Some(spec) = state.specs.latest(&page.slug).await? else {
        warn!("game {} has a page but no rules; leaving it out", page.slug);
        return Ok(None);
    };
    match spec.seats() {
        Ok(seats) => Ok(Some(Playable { page, spec, seats })),
        Err(err) => {
            warn!(
                "game {} has rules the engine cannot read ({err}); leaving it out",
                page.slug
            );
            Ok(None)
        }
    }
}
