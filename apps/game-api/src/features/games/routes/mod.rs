use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use modern_chess::{
    logic::{Game, GamePhase, GameTransition},
    shared::EffectMetadata,
    specs::GameSpec,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{
    features::{
        common::{
            bad_request, conflict, db_pool, forbidden, internal_error, map_db_error, not_found,
            ApiError,
        },
        games::repository::GamesRepository,
        specs::repository::SpecsRepository,
    },
    infrastructure::db::models::{GameInstanceRecord, GamePlayerRecord, StoredGameSpec},
    AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_game))
        .route("/{id}", get(get_game))
        .route("/{id}/join", post(join_game))
        .route("/{id}/legal-moves", post(legal_moves))
        .route("/{id}/legal-drops", post(legal_drops))
        .route("/{id}/move", post(move_piece))
        .route("/{id}/drop", post(drop_piece))
}

#[derive(Debug, Deserialize)]
struct CreateGameRequest {
    spec_slug: String,
    spec_version: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct JoinGameRequest {
    user_id: String,
    seat: String,
}

#[derive(Debug, Deserialize)]
struct LegalMovesRequest {
    from: Vec<u8>,
}

#[derive(Debug, Deserialize)]
struct LegalDropsRequest {
    piece_code: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct MoveRequest {
    actor_user_id: String,
    from: Vec<u8>,
    to: Vec<u8>,
    promotion: Option<String>,
    expected_revision: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
struct DropRequest {
    actor_user_id: String,
    piece_code: String,
    to: Vec<u8>,
    expected_revision: Option<i64>,
}

#[derive(Debug, Serialize)]
struct GameResponse {
    id: Uuid,
    spec_slug: String,
    spec_version: i32,
    spec_name: String,
    status: String,
    revision: i64,
    current_player: String,
    state: Value,
    players: Vec<GamePlayerRecord>,
}

#[derive(Debug, Serialize)]
struct LegalMove {
    to: Vec<u8>,
    action: String,
    options: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct LegalMovesResponse {
    moves: Vec<LegalMove>,
}

#[derive(Debug, Serialize)]
struct LegalDropsResponse {
    piece_code: String,
    squares: Vec<Vec<u8>>,
}

async fn create_game(
    State(state): State<AppState>,
    Json(payload): Json<CreateGameRequest>,
) -> Result<(StatusCode, Json<GameResponse>), ApiError> {
    let pool = db_pool(&state)?;

    let specs_repo = SpecsRepository::new(pool.clone());
    let games_repo = GamesRepository::new(pool.clone());

    let spec = specs_repo
        .resolve_spec(&payload.spec_slug, payload.spec_version)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| not_found("spec"))?;

    let game = game_from_spec(&spec, None)?;
    let state_json = state_json_from_game(&game)?;

    let instance = games_repo
        .create_instance(&spec, state_json)
        .await
        .map_err(map_db_error)?;

    let response = to_game_response(&instance, &spec, vec![], &game)?;
    Ok((StatusCode::CREATED, Json(response)))
}

async fn get_game(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<GameResponse>, ApiError> {
    let context = load_game_context(&state, id).await?;
    let response = to_game_response(
        &context.instance,
        &context.spec,
        context.players,
        &context.game,
    )?;
    Ok(Json(response))
}

async fn join_game(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<JoinGameRequest>,
) -> Result<Json<GameResponse>, ApiError> {
    let pool = db_pool(&state)?;
    let repo = GamesRepository::new(pool.clone());

    let mut tx = repo
        .pool()
        .begin()
        .await
        .map_err(|err| map_db_error(err.into()))?;

    let instance = repo
        .get_instance_for_update(&mut tx, id)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| not_found("game"))?;

    if instance.status == "FINISHED" {
        tx.rollback()
            .await
            .map_err(|err| map_db_error(err.into()))?;
        return Err(conflict("game is already finished"));
    }

    let spec = repo
        .get_spec_by_id_tx(&mut tx, instance.spec_id)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| not_found("spec"))?;

    let seats = seats_from_spec(&spec)?;
    if !seats.iter().any(|seat| seat == &payload.seat) {
        tx.rollback()
            .await
            .map_err(|err| map_db_error(err.into()))?;
        return Err(bad_request(format!(
            "invalid seat '{}': not part of this game spec",
            payload.seat
        )));
    }

    if let Some(existing) = repo
        .find_player_by_seat_tx(&mut tx, id, &payload.seat)
        .await
        .map_err(map_db_error)?
    {
        if existing.user_id != payload.user_id {
            tx.rollback()
                .await
                .map_err(|err| map_db_error(err.into()))?;
            return Err(conflict(format!(
                "seat '{}' is already taken",
                payload.seat
            )));
        }
    } else {
        if let Some(other_seat) = repo
            .find_seat_by_user_tx(&mut tx, id, &payload.user_id)
            .await
            .map_err(map_db_error)?
        {
            if other_seat != payload.seat {
                tx.rollback()
                    .await
                    .map_err(|err| map_db_error(err.into()))?;
                return Err(conflict(format!(
                    "user is already joined as '{}', cannot also join as '{}'",
                    other_seat, payload.seat
                )));
            }
        }

        repo.insert_player_tx(&mut tx, id, &payload.seat, &payload.user_id)
            .await
            .map_err(map_db_error)?;
    }

    let joined_count = repo
        .count_players_tx(&mut tx, id)
        .await
        .map_err(map_db_error)?;
    if instance.status == "LOBBY" && joined_count >= seats.len() as i64 {
        repo.update_status_tx(&mut tx, id, "ACTIVE")
            .await
            .map_err(map_db_error)?;
    }

    tx.commit().await.map_err(|err| map_db_error(err.into()))?;

    let context = load_game_context(&state, id).await?;
    let response = to_game_response(
        &context.instance,
        &context.spec,
        context.players,
        &context.game,
    )?;
    Ok(Json(response))
}

async fn legal_moves(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<LegalMovesRequest>,
) -> Result<Json<LegalMovesResponse>, ApiError> {
    let context = load_game_context(&state, id).await?;
    if context.instance.status != "ACTIVE" {
        return Err(conflict("game is not active"));
    }

    let mut game = context.game;
    game.transition(GameTransition::CalculateMoves {
        position: payload.from,
    })
    .map_err(|err| bad_request(format!("cannot calculate moves: {err:?}")))?;

    let mut moves: Vec<LegalMove> = game
        .state
        .available_moves
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|(to, effect)| LegalMove {
            to,
            action: effect.action,
            options: match effect.metadata {
                Some(EffectMetadata::Options(v)) => Some(v),
                None => None,
            },
        })
        .collect();

    moves.sort_by(|a, b| a.to.cmp(&b.to));

    Ok(Json(LegalMovesResponse { moves }))
}

async fn legal_drops(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<LegalDropsRequest>,
) -> Result<Json<LegalDropsResponse>, ApiError> {
    let context = load_game_context(&state, id).await?;
    if context.instance.status != "ACTIVE" {
        return Err(conflict("game is not active"));
    }

    let mut game = context.game;
    game.transition(GameTransition::CalculateDrops {
        piece_code: payload.piece_code.clone(),
    })
    .map_err(|err| bad_request(format!("cannot calculate drops: {err:?}")))?;

    let mut squares: Vec<Vec<u8>> = game
        .state
        .available_moves
        .clone()
        .unwrap_or_default()
        .into_keys()
        .collect();

    squares.sort();

    Ok(Json(LegalDropsResponse {
        piece_code: payload.piece_code,
        squares,
    }))
}

async fn move_piece(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<MoveRequest>,
) -> Result<Json<GameResponse>, ApiError> {
    let pool = db_pool(&state)?;
    let repo = GamesRepository::new(pool.clone());

    let mut tx = repo
        .pool()
        .begin()
        .await
        .map_err(|err| map_db_error(err.into()))?;

    let instance = repo
        .get_instance_for_update(&mut tx, id)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| not_found("game"))?;

    if instance.status != "ACTIVE" {
        return Err(reject_with_log(
            &repo,
            tx,
            id,
            &payload.actor_user_id,
            "MOVE",
            &payload,
            "game is not active".to_string(),
            conflict("game is not active"),
        )
        .await);
    }

    if let Some(expected_revision) = payload.expected_revision {
        if expected_revision != instance.revision {
            return Err(reject_with_log(
                &repo,
                tx,
                id,
                &payload.actor_user_id,
                "MOVE",
                &payload,
                format!(
                    "revision mismatch: expected {}, actual {}",
                    expected_revision, instance.revision
                ),
                conflict("revision mismatch"),
            )
            .await);
        }
    }

    let spec = repo
        .get_spec_by_id_tx(&mut tx, instance.spec_id)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| not_found("spec"))?;

    let players = repo
        .list_players_tx(&mut tx, id)
        .await
        .map_err(map_db_error)?;

    let mut game = game_from_spec(&spec, Some(&instance.state_json))?;

    ensure_actor_owns_current_turn(&players, &game.current_player(), &payload.actor_user_id)?;

    if let Err(err) = game.transition(GameTransition::CalculateMoves {
        position: payload.from.clone(),
    }) {
        return Err(reject_with_log(
            &repo,
            tx,
            id,
            &payload.actor_user_id,
            "MOVE",
            &payload,
            format!("calculate move failed: {err:?}"),
            bad_request(format!("invalid source square: {err:?}")),
        )
        .await);
    }

    if let Err(err) = game.transition(GameTransition::ExecuteMove {
        position: payload.to.clone(),
    }) {
        return Err(reject_with_log(
            &repo,
            tx,
            id,
            &payload.actor_user_id,
            "MOVE",
            &payload,
            format!("execute move failed: {err:?}"),
            bad_request(format!("invalid target square: {err:?}")),
        )
        .await);
    }

    if let GamePhase::Transforming { options, .. } = &game.state.phase {
        match payload.promotion.as_ref() {
            Some(target) => {
                if let Err(err) = game.transition(GameTransition::Transform {
                    target: target.clone(),
                }) {
                    return Err(reject_with_log(
                        &repo,
                        tx,
                        id,
                        &payload.actor_user_id,
                        "MOVE",
                        &payload,
                        format!("transform failed: {err:?}"),
                        bad_request(format!("invalid promotion choice: {err:?}")),
                    )
                    .await);
                }
            }
            None => {
                return Err(reject_with_log(
                    &repo,
                    tx,
                    id,
                    &payload.actor_user_id,
                    "MOVE",
                    &payload,
                    format!("promotion required; options={options:?}"),
                    conflict(format!("promotion required; choose one of: {:?}", options)),
                )
                .await);
            }
        }
    }

    let next_state = state_json_from_game(&game)?;
    let next_status = if matches!(game.state.phase, GamePhase::GameOver { .. }) {
        "FINISHED"
    } else {
        "ACTIVE"
    };
    let next_revision = instance.revision + 1;

    repo.update_state_tx(&mut tx, id, next_state.clone(), next_status, next_revision)
        .await
        .map_err(map_db_error)?;

    let seq = repo
        .next_command_seq_tx(&mut tx, id)
        .await
        .map_err(map_db_error)?;
    repo.insert_command_tx(
        &mut tx,
        id,
        seq,
        &payload.actor_user_id,
        "MOVE",
        serde_json::to_value(&payload).map_err(|err| internal_error(err.to_string()))?,
        true,
        None,
    )
    .await
    .map_err(map_db_error)?;

    tx.commit().await.map_err(|err| map_db_error(err.into()))?;

    let updated_instance = GameInstanceRecord {
        id: instance.id,
        spec_id: instance.spec_id,
        spec_slug: instance.spec_slug,
        spec_version: instance.spec_version,
        status: next_status.to_string(),
        state_json: next_state,
        revision: next_revision,
    };

    let response = to_game_response(&updated_instance, &spec, players, &game)?;
    Ok(Json(response))
}

async fn drop_piece(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<DropRequest>,
) -> Result<Json<GameResponse>, ApiError> {
    let pool = db_pool(&state)?;
    let repo = GamesRepository::new(pool.clone());

    let mut tx = repo
        .pool()
        .begin()
        .await
        .map_err(|err| map_db_error(err.into()))?;

    let instance = repo
        .get_instance_for_update(&mut tx, id)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| not_found("game"))?;

    if instance.status != "ACTIVE" {
        return Err(reject_with_log(
            &repo,
            tx,
            id,
            &payload.actor_user_id,
            "DROP",
            &payload,
            "game is not active".to_string(),
            conflict("game is not active"),
        )
        .await);
    }

    if let Some(expected_revision) = payload.expected_revision {
        if expected_revision != instance.revision {
            return Err(reject_with_log(
                &repo,
                tx,
                id,
                &payload.actor_user_id,
                "DROP",
                &payload,
                format!(
                    "revision mismatch: expected {}, actual {}",
                    expected_revision, instance.revision
                ),
                conflict("revision mismatch"),
            )
            .await);
        }
    }

    let spec = repo
        .get_spec_by_id_tx(&mut tx, instance.spec_id)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| not_found("spec"))?;

    let players = repo
        .list_players_tx(&mut tx, id)
        .await
        .map_err(map_db_error)?;

    let mut game = game_from_spec(&spec, Some(&instance.state_json))?;

    ensure_actor_owns_current_turn(&players, &game.current_player(), &payload.actor_user_id)?;

    if let Err(err) = game.transition(GameTransition::CalculateDrops {
        piece_code: payload.piece_code.clone(),
    }) {
        return Err(reject_with_log(
            &repo,
            tx,
            id,
            &payload.actor_user_id,
            "DROP",
            &payload,
            format!("calculate drops failed: {err:?}"),
            bad_request(format!("invalid drop piece: {err:?}")),
        )
        .await);
    }

    if let Err(err) = game.transition(GameTransition::ExecuteDrop {
        position: payload.to.clone(),
    }) {
        return Err(reject_with_log(
            &repo,
            tx,
            id,
            &payload.actor_user_id,
            "DROP",
            &payload,
            format!("execute drop failed: {err:?}"),
            bad_request(format!("invalid drop square: {err:?}")),
        )
        .await);
    }

    let next_state = state_json_from_game(&game)?;
    let next_status = if matches!(game.state.phase, GamePhase::GameOver { .. }) {
        "FINISHED"
    } else {
        "ACTIVE"
    };
    let next_revision = instance.revision + 1;

    repo.update_state_tx(&mut tx, id, next_state.clone(), next_status, next_revision)
        .await
        .map_err(map_db_error)?;

    let seq = repo
        .next_command_seq_tx(&mut tx, id)
        .await
        .map_err(map_db_error)?;
    repo.insert_command_tx(
        &mut tx,
        id,
        seq,
        &payload.actor_user_id,
        "DROP",
        serde_json::to_value(&payload).map_err(|err| internal_error(err.to_string()))?,
        true,
        None,
    )
    .await
    .map_err(map_db_error)?;

    tx.commit().await.map_err(|err| map_db_error(err.into()))?;

    let updated_instance = GameInstanceRecord {
        id: instance.id,
        spec_id: instance.spec_id,
        spec_slug: instance.spec_slug,
        spec_version: instance.spec_version,
        status: next_status.to_string(),
        state_json: next_state,
        revision: next_revision,
    };

    let response = to_game_response(&updated_instance, &spec, players, &game)?;
    Ok(Json(response))
}

struct GameContext {
    instance: GameInstanceRecord,
    spec: StoredGameSpec,
    players: Vec<GamePlayerRecord>,
    game: Game,
}

async fn load_game_context(state: &AppState, id: Uuid) -> Result<GameContext, ApiError> {
    let pool = db_pool(state)?;
    let repo = GamesRepository::new(pool.clone());

    let instance = repo
        .get_instance(id)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| not_found("game"))?;
    let spec = repo
        .resolve_spec_for_game(id)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| not_found("spec"))?;
    let players = repo.list_players(id).await.map_err(map_db_error)?;
    let game = game_from_spec(&spec, Some(&instance.state_json))?;

    Ok(GameContext {
        instance,
        spec,
        players,
        game,
    })
}

fn to_game_response(
    instance: &GameInstanceRecord,
    spec: &StoredGameSpec,
    players: Vec<GamePlayerRecord>,
    game: &Game,
) -> Result<GameResponse, ApiError> {
    Ok(GameResponse {
        id: instance.id,
        spec_slug: instance.spec_slug.clone(),
        spec_version: instance.spec_version,
        spec_name: spec.name.clone(),
        status: instance.status.clone(),
        revision: instance.revision,
        current_player: game.current_player(),
        state: state_json_from_game(game)?,
        players,
    })
}

fn game_from_spec(spec: &StoredGameSpec, state_json: Option<&Value>) -> Result<Game, ApiError> {
    let game_spec: GameSpec = serde_json::from_value(spec.rules_json.clone())
        .map_err(|err| internal_error(err.to_string()))?;
    game_spec
        .validate_specs()
        .map_err(|err| internal_error(err.to_string()))?;

    let mut game = Game::from_spec(game_spec);

    if let Some(state_json) = state_json {
        game.restore_state(&state_json.to_string())
            .map_err(|err| internal_error(err.to_string()))?;
    }

    Ok(game)
}

fn seats_from_spec(spec: &StoredGameSpec) -> Result<Vec<String>, ApiError> {
    let game_spec: GameSpec = serde_json::from_value(spec.rules_json.clone())
        .map_err(|err| internal_error(err.to_string()))?;
    game_spec
        .validate_specs()
        .map_err(|err| internal_error(err.to_string()))?;

    Ok(game_spec
        .players
        .into_iter()
        .map(|player| player.name)
        .collect())
}

fn state_json_from_game(game: &Game) -> Result<Value, ApiError> {
    let state = game
        .save_state()
        .map_err(|err| internal_error(err.to_string()))?;
    serde_json::from_str(&state).map_err(|err| internal_error(err.to_string()))
}

fn ensure_actor_owns_current_turn(
    players: &[GamePlayerRecord],
    current_player: &str,
    actor_user_id: &str,
) -> Result<(), ApiError> {
    let map: HashMap<&str, &str> = players
        .iter()
        .map(|player| (player.seat.as_str(), player.user_id.as_str()))
        .collect();

    let Some(owner) = map.get(current_player) else {
        return Err(conflict(format!(
            "current turn seat '{}' is not joined",
            current_player
        )));
    };

    if owner != &actor_user_id {
        return Err(forbidden(format!(
            "actor does not control current seat '{}'",
            current_player
        )));
    }

    Ok(())
}

async fn reject_with_log<T: Serialize>(
    repo: &GamesRepository,
    mut tx: Transaction<'_, Postgres>,
    game_id: Uuid,
    actor: &str,
    command_type: &str,
    payload: &T,
    message: String,
    response: ApiError,
) -> ApiError {
    let seq = match repo.next_command_seq_tx(&mut tx, game_id).await {
        Ok(seq) => seq,
        Err(err) => return map_db_error(err),
    };

    let payload_json = match serde_json::to_value(payload) {
        Ok(value) => value,
        Err(err) => return internal_error(err.to_string()),
    };

    if let Err(err) = repo
        .insert_command_tx(
            &mut tx,
            game_id,
            seq,
            actor,
            command_type,
            payload_json,
            false,
            Some(message),
        )
        .await
    {
        return map_db_error(err);
    }

    if let Err(err) = tx.commit().await {
        return map_db_error(err.into());
    }

    response
}
