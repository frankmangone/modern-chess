use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde_json::Value;
use tracing::warn;

use super::error::ApiError;
use super::GameState;
use crate::store::StoreError;
use crate::{GameInstance, GamePage, GameSpec, InstanceId, Player, SeatName, Slug, Variant};

/// One card on the front page: a game, with the variants it can be played in.
#[derive(Serialize)]
pub struct CatalogEntry {
    slug: Slug,
    name: String,
    variants: Vec<VariantLabel>,
    tagline: String,
    tags: Vec<String>,
    accent: Option<String>,
    cover_image: Option<String>,
    featured: bool,
}

#[derive(Serialize)]
pub struct VariantLabel {
    slug: Slug,
    label: String,
}

/// What the game page needs: its tunable page, the rules to run a preview with, the variants
/// to choose from, and the lobbies waiting for players in any of them.
#[derive(Serialize)]
pub struct GameDetail {
    page: GamePage,
    /// The rules of the preview variant, kept for clients that only run a preview.
    spec: SpecSummary,
    preview_variant: Slug,
    variants: Vec<VariantDetail>,
    open_lobbies: Vec<LobbySummary>,
}

#[derive(Serialize)]
pub struct VariantDetail {
    slug: Slug,
    label: String,
    /// The name of the variant's rules ("Go 13x13").
    name: String,
    seats: Vec<SeatName>,
    /// A skin that replaces the game's, or null when the variant uses the game's.
    skin: Option<Value>,
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
    /// The variant (its slug and label) the lobby will be played in.
    variant: Slug,
    variant_label: String,
    capacity: usize,
    players: Vec<LobbyPlayer>,
    created_at: DateTime<Utc>,
}

impl LobbySummary {
    fn new(lobby: &GameInstance, variant: &Variant) -> Self {
        Self {
            id: lobby.id(),
            variant: variant.slug.clone(),
            variant_label: variant.label.clone(),
            capacity: lobby.capacity(),
            players: lobby.players().iter().map(LobbyPlayer::from).collect(),
            created_at: lobby.created_at(),
        }
    }
}

#[derive(Serialize)]
pub struct LobbyPlayer {
    seat: SeatName,
    handle: String,
}

impl From<&Player> for LobbyPlayer {
    fn from(player: &Player) -> Self {
        Self {
            seat: player.seat.clone(),
            handle: player.handle.to_string(),
        }
    }
}

/// Lobbies older than this are not offered: nobody is waiting in them any more.
const OPEN_LOBBY_MAX_AGE_HOURS: i64 = 24;
const OPEN_LOBBY_LIMIT: i64 = 50;

/// A variant that can be played: its rules exist and the engine can still read them.
pub(super) struct PlayableVariant {
    pub variant: Variant,
    pub spec: GameSpec,
    pub seats: Vec<SeatName>,
}

/// A game that can be shown: it has a page and at least one variant that can be played.
pub(super) struct Game {
    page: GamePage,
    name: String,
    variants: Vec<PlayableVariant>,
    /// Index into `variants` of the one "Try it" runs.
    preview: usize,
}

pub async fn list(State(state): State<GameState>) -> Result<Json<Vec<CatalogEntry>>, ApiError> {
    let mut by_game: HashMap<String, Vec<Variant>> = HashMap::new();
    for variant in state.variants.all().await? {
        by_game
            .entry(variant.game.as_str().to_string())
            .or_default()
            .push(variant);
    }

    let mut games = Vec::new();
    for page in state.pages.list().await? {
        let variants = by_game.remove(page.slug.as_str()).unwrap_or_default();
        if let Some(game) = playable_game(&state, page, variants).await? {
            games.push(game);
        }
    }
    games.sort_by(|a, b| (a.page.sort_order, &a.name).cmp(&(b.page.sort_order, &b.name)));

    Ok(Json(
        games
            .into_iter()
            .map(|game| CatalogEntry {
                variants: game
                    .variants
                    .iter()
                    .map(|v| VariantLabel {
                        slug: v.variant.slug.clone(),
                        label: v.variant.label.clone(),
                    })
                    .collect(),
                slug: game.page.slug,
                name: game.name,
                tagline: game.page.tagline,
                tags: game.page.tags,
                accent: game.page.accent,
                cover_image: state.images.url(game.page.cover_image),
                featured: game.page.featured,
            })
            .collect(),
    ))
}

pub async fn detail(
    State(state): State<GameState>,
    Path(game): Path<String>,
) -> Result<Json<GameDetail>, ApiError> {
    let not_found = || ApiError::not_found("game not found");
    let game = Slug::parse(&game).map_err(|_| not_found())?;
    let game = find_game(&state, &game).await?.ok_or_else(not_found)?;
    let cutoff = Utc::now() - Duration::hours(OPEN_LOBBY_MAX_AGE_HOURS);
    let slugs: Vec<Slug> = game
        .variants
        .iter()
        .map(|v| v.variant.slug.clone())
        .collect();
    let lobbies = state
        .instances
        .open_lobbies(&slugs, cutoff, OPEN_LOBBY_LIMIT)
        .await?;
    let open_lobbies = lobbies
        .iter()
        .filter_map(|lobby| {
            let variant = game
                .variants
                .iter()
                .find(|v| &v.variant.slug == lobby.slug())?;
            Some(LobbySummary::new(lobby, &variant.variant))
        })
        .collect();

    let Game {
        mut page,
        name,
        variants,
        preview,
    } = game;
    let preview_slug = variants[preview].variant.slug.clone();
    let spec = SpecSummary {
        name: variants[preview].spec.name.clone(),
        version: variants[preview].spec.version,
        seats: variants[preview].seats.clone(),
        rules: variants[preview].spec.rules.clone(),
    };
    page.cover_image = state.images.url(page.cover_image);
    page.name = Some(name);
    page.preview_variant = Some(preview_slug.clone());
    let variants = variants
        .into_iter()
        .map(|v| VariantDetail {
            skin: v
                .variant
                .skin
                .filter(|skin| Some(skin) != page.skin.as_ref()),
            slug: v.variant.slug,
            label: v.variant.label,
            name: v.spec.name,
            seats: v.seats,
        })
        .collect();

    Ok(Json(GameDetail {
        page,
        spec,
        preview_variant: preview_slug,
        variants,
        open_lobbies,
    }))
}

/// The game called `game`, if it can be shown.
pub(super) async fn find_game(state: &GameState, game: &Slug) -> Result<Option<Game>, StoreError> {
    match state.pages.get(game).await? {
        Some(page) => {
            let variants = state.variants.for_game(game).await?;
            playable_game(state, page, variants).await
        }
        None => Ok(None),
    }
}

/// The variant called `variant`, if it can be played. It needs no page of its own.
pub(super) async fn find_variant(
    state: &GameState,
    variant: &Slug,
) -> Result<Option<PlayableVariant>, StoreError> {
    match state.variants.get(variant).await? {
        Some(variant) => playable_variant(state, variant).await,
        None => Ok(None),
    }
}

/// Pairs a page with the variants of its game that can be played. A variant whose rules are
/// missing, or that the engine can no longer read, is left out and logged, and so is a game
/// with none left, so one bad variant never takes the whole catalogue down.
async fn playable_game(
    state: &GameState,
    page: GamePage,
    variants: Vec<Variant>,
) -> Result<Option<Game>, StoreError> {
    let mut playable = Vec::new();
    for variant in variants {
        if let Some(found) = playable_variant(state, variant).await? {
            playable.push(found);
        }
    }
    if playable.is_empty() {
        warn!(
            "game {} has no variant that can be played; leaving it out",
            page.slug
        );
        return Ok(None);
    }
    let preview = page
        .preview_variant
        .as_ref()
        .and_then(|wanted| playable.iter().position(|v| &v.variant.slug == wanted))
        .unwrap_or(0);
    let name = page
        .name
        .clone()
        .unwrap_or_else(|| playable[preview].spec.name.clone());
    Ok(Some(Game {
        page,
        name,
        variants: playable,
        preview,
    }))
}

async fn playable_variant(
    state: &GameState,
    variant: Variant,
) -> Result<Option<PlayableVariant>, StoreError> {
    let Some(spec) = state.specs.latest(&variant.slug).await? else {
        warn!("variant {} has no rules; leaving it out", variant.slug);
        return Ok(None);
    };
    match spec.seats() {
        Ok(seats) => Ok(Some(PlayableVariant {
            variant,
            spec,
            seats,
        })),
        Err(err) => {
            warn!(
                "variant {} has rules the engine cannot read ({err}); leaving it out",
                variant.slug
            );
            Ok(None)
        }
    }
}
