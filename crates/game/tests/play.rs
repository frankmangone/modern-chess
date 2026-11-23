//! Playing a match over HTTP, against a real Postgres (`make db-up`). The engine decides what
//! is legal; these tests check who may ask, in what order, and what is kept.

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use chrono::{DateTime, Duration, TimeZone, Utc};
use modern_chess_game::seed;
use modern_chess_game::store::{CommandRepo, InstanceRepo, PageRepo, SpecRepo};
use modern_chess_game::{router, GameInstance, GameState, InstanceId, Slug};
use modern_chess_kernel::{Handle, TokenVerifier, UserId};
use modern_chess_testdb::with_db;
use modern_chess_testkeys::TestKeys;
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::task::JoinSet;
use tower::ServiceExt;

const PROMOTE: &str = include_str!("fixtures/promote.json");
const WIN: &str = include_str!("fixtures/win.json");
const DRAW: &str = include_str!("fixtures/draw.json");

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap()
}

struct World {
    app: Router,
    keys: TestKeys,
    pool: PgPool,
}

#[derive(Clone)]
struct Player {
    id: UserId,
    handle: String,
    token: String,
}

async fn world(pool: PgPool) -> World {
    modern_chess_game::migrate(&pool).await.unwrap();
    let specs = SpecRepo::new(pool.clone());
    let pages = PageRepo::new(pool.clone());
    seed::run(&specs, &pages, t0()).await.unwrap();
    let keys = TestKeys::generate();
    let verifier = TokenVerifier::from_public_pem(&keys.public_pem).unwrap();
    World {
        app: router(GameState::new(pool.clone(), verifier)),
        keys,
        pool,
    }
}

impl World {
    fn player(&self, handle: &str) -> Player {
        let id = UserId::new();
        let token = self
            .keys
            .token_for(id, handle, (Utc::now() + Duration::hours(1)).timestamp());
        Player {
            id,
            handle: handle.to_string(),
            token,
        }
    }

    async fn call(
        &self,
        method: Method,
        uri: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        call(&self.app, method, uri, token, body).await
    }

    /// A started match of the stored game `slug`, white and black already seated.
    async fn started(&self, slug: &str) -> (String, Player, Player) {
        let spec = SpecRepo::new(self.pool.clone())
            .latest(&Slug::parse(slug).unwrap())
            .await
            .unwrap()
            .expect("the game is seeded");
        self.started_from(spec).await
    }

    /// A started match of a game with these rules, stored under its own slug.
    async fn started_with(&self, slug: &str, rules: &str) -> (String, Player, Player) {
        let spec = SpecRepo::new(self.pool.clone())
            .upsert(
                Slug::parse(slug).unwrap(),
                slug.to_string(),
                serde_json::from_str(rules).unwrap(),
                t0(),
            )
            .await
            .unwrap();
        self.started_from(spec).await
    }

    async fn started_from(&self, spec: modern_chess_game::GameSpec) -> (String, Player, Player) {
        let (first, second) = (self.player("Alice"), self.player("Bobby"));
        let mut lobby = GameInstance::new_lobby(&spec, t0()).unwrap();
        let seats = lobby.seats().to_vec();
        for (player, seat) in [(&first, &seats[0]), (&second, &seats[1])] {
            lobby
                .join(
                    player.id,
                    Handle::parse(&player.handle).unwrap(),
                    seat.clone(),
                    t0(),
                )
                .unwrap();
        }
        InstanceRepo::new(self.pool.clone())
            .create(&lobby)
            .await
            .unwrap();
        (lobby.id().as_uuid().to_string(), first, second)
    }

    async fn send(&self, id: &str, by: &Player, revision: u64, kind: Value) -> (StatusCode, Value) {
        self.call(
            Method::POST,
            &format!("/instances/{id}/commands"),
            Some(&by.token),
            Some(json!({ "revision": revision, "kind": kind })),
        )
        .await
    }

    async fn snapshot(&self, id: &str, token: Option<&str>) -> Value {
        let (status, body) = self
            .call(Method::GET, &format!("/instances/{id}"), token, None)
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    }

    async fn log(&self, id: &str) -> Vec<modern_chess_game::Command> {
        let id = InstanceId::from_uuid(id.parse().unwrap());
        CommandRepo::new(self.pool.clone()).list(id).await.unwrap()
    }
}

async fn call(
    app: &Router,
    method: Method,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let body = match body {
        Some(json) => {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// The revision a two-seat match has when it starts: one for each player's join.
const START: u64 = 2;

/// Answers with a place: clicking a piece, or a destination.
fn at(name: &str) -> Value {
    json!({ "type": "choose", "place": name })
}

fn hand(code: &str) -> Value {
    json!({ "type": "choose", "hand": code })
}

fn code(code: &str) -> Value {
    json!({ "type": "choose", "code": code })
}

fn cancel() -> Value {
    json!({ "type": "cancel" })
}

#[tokio::test]
async fn only_the_seat_to_move_may_choose() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, black) = world.started("chess").await;

        let (status, body) = world.send(&id, &black, START, at("4,6")).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body, json!({ "error": "it is not your turn" }));
        assert_eq!(world.snapshot(&id, None).await["revision"], START);
        assert!(world.log(&id).await.is_empty());

        let (status, body) = world.send(&id, &white, START, at("4,1")).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["view"]["choice"]["start"], json!("4,1"));
    })
    .await;
}

#[tokio::test]
async fn move_advances_the_turn_and_bumps_the_revision() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, black) = world.started("chess").await;

        let (_, picked) = world.send(&id, &white, START, at("4,1")).await;
        assert_eq!(picked["revision"], START + 1);
        assert_eq!(
            picked["view"]["choice"]["places"],
            json!([{ "name": "4,2", "for": "MOVE" }, { "name": "4,3", "for": "MOVE" }])
        );

        let (status, moved) = world.send(&id, &white, START + 1, at("4,3")).await;
        assert_eq!(status, StatusCode::OK, "{moved}");
        assert_eq!(moved["revision"], START + 2);
        assert_eq!(moved["status"], "active");
        assert_eq!(moved["view"]["turn"], "BLACK");
        assert_eq!(moved["you"], "WHITE");

        // The opponent sees the same position, and now it is their turn.
        let seen = world.snapshot(&id, Some(&black.token)).await;
        // Only the player to move sees a choice, so that is the one thing that differs.
        let (mut theirs, mut mine) = (seen["view"].clone(), moved["view"].clone());
        assert_eq!(mine["choice"]["places"], json!([]));
        assert!(!theirs["choice"]["places"].as_array().unwrap().is_empty());
        theirs["choice"] = Value::Null;
        mine["choice"] = Value::Null;
        assert_eq!(theirs, mine);
        assert_eq!(seen["you"], "BLACK");
        let (status, _) = world.send(&id, &black, START + 2, at("4,6")).await;
        assert_eq!(status, StatusCode::OK);

        let log = world.log(&id).await;
        assert_eq!(log.len(), 3);
        assert!(log.iter().all(|c| c.accepted));
    })
    .await;
}

#[tokio::test]
async fn a_stale_revision_is_409_and_changes_nothing() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;
        let before = world.snapshot(&id, None).await;

        for revision in [START + 1, START + 5] {
            let (status, body) = world.send(&id, &white, revision, at("4,1")).await;
            assert_eq!(status, StatusCode::CONFLICT, "{body}");
            assert_eq!(body, json!({ "error": "the position has changed" }));
        }

        assert_eq!(world.snapshot(&id, None).await, before);
        assert!(world.log(&id).await.is_empty());
    })
    .await;
}

#[tokio::test]
async fn the_same_command_sent_twice_applies_once() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;
        world.send(&id, &white, START, at("4,1")).await;
        world.send(&id, &white, START + 1, at("4,3")).await;

        // A double click: the same move, still based on the revision before it.
        let (status, _) = world.send(&id, &white, START + 1, at("4,3")).await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(world.snapshot(&id, None).await["revision"], START + 2);
        assert_eq!(world.log(&id).await.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn two_identical_commands_at_the_same_moment_apply_once() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;

        let mut racing = JoinSet::new();
        for _ in 0..8 {
            let (app, id, token) = (world.app.clone(), id.clone(), white.token.clone());
            racing.spawn(async move {
                call(
                    &app,
                    Method::POST,
                    &format!("/instances/{id}/commands"),
                    Some(&token),
                    Some(json!({ "revision": START, "kind": at("4,1") })),
                )
                .await
                .0
            });
        }
        let mut statuses = Vec::new();
        while let Some(done) = racing.join_next().await {
            statuses.push(done.unwrap());
        }

        assert_eq!(statuses.iter().filter(|s| **s == StatusCode::OK).count(), 1);
        assert_eq!(
            statuses
                .iter()
                .filter(|s| **s == StatusCode::CONFLICT)
                .count(),
            7
        );
        assert_eq!(world.snapshot(&id, None).await["revision"], START + 1);
        assert_eq!(world.log(&id).await.len(), 1);
    })
    .await;
}

/// A command locks the match in a transaction, so it holds one connection while it decides.
/// It must not ask for a second one meanwhile: with every connection busy that way, the
/// requests would wait for each other until the pool gave up.
#[tokio::test]
async fn a_command_needs_only_one_connection_at_a_time() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;

        let tight = sqlx::pool::PoolOptions::<sqlx::Postgres>::new()
            .max_connections(2)
            .acquire_timeout(std::time::Duration::from_secs(1))
            .connect_with((*world.pool.connect_options()).clone())
            .await
            .unwrap();
        let verifier = TokenVerifier::from_public_pem(&world.keys.public_pem).unwrap();
        let app = router(GameState::new(tight.clone(), verifier));
        let _busy = tight.acquire().await.unwrap();

        let (status, body) = call(
            &app,
            Method::POST,
            &format!("/instances/{id}/commands"),
            Some(&white.token),
            Some(json!({ "revision": START, "kind": at("4,1") })),
        )
        .await;

        assert_eq!(status, StatusCode::OK, "{body}");
    })
    .await;
}

#[tokio::test]
async fn an_illegal_answer_is_422_and_logged_as_rejected() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;
        world.send(&id, &white, START, at("4,1")).await;

        // A piece is picked up, so a code answers nothing.
        let (status, body) = world.send(&id, &white, START + 1, code("QUEEN")).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        let message = body["error"].as_str().unwrap();
        assert!(!message.is_empty() && !message.contains('{'), "{message}");
        let seen = world.snapshot(&id, Some(&white.token)).await;
        assert_eq!(
            seen["revision"],
            START + 1,
            "a refused answer changes nothing"
        );
        assert_eq!(seen["view"]["choice"]["start"], json!("4,1"));
        let log = world.log(&id).await;
        assert_eq!(log.len(), 2);
        assert!(log[0].accepted);
        assert!(!log[1].accepted);
        assert_eq!(log[1].actor, white.id);
        assert!(log[1].error.is_some());

        // With nothing picked up, an empty square is refused the same way.
        world.send(&id, &white, START + 1, cancel()).await;
        let (status, _) = world.send(&id, &white, START + 2, at("4,4")).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(world.log(&id).await.len(), 4);
    })
    .await;
}

#[tokio::test]
async fn clicking_a_place_that_answers_nothing_only_puts_the_piece_back() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;
        world.send(&id, &white, START, at("4,1")).await;

        let (status, body) = world.send(&id, &white, START + 1, at("4,5")).await;

        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["view"]["choice"]["chosen"], json!([]));
        assert_eq!(body["view"]["turn"], "WHITE");
    })
    .await;
}

#[tokio::test]
async fn a_promotion_is_a_choose_code_command() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started_with("promote", PROMOTE).await;

        world.send(&id, &white, START, at("0,2")).await;
        let (_, asked) = world.send(&id, &white, START + 1, at("0,3")).await;
        assert_eq!(asked["view"]["choice"]["codes"], json!(["QUEEN", "ROOK"]));
        assert_eq!(asked["view"]["turn"], "WHITE");

        let (status, done) = world.send(&id, &white, START + 2, code("QUEEN")).await;

        assert_eq!(status, StatusCode::OK, "{done}");
        assert_eq!(done["view"]["choice"]["codes"], json!([]));
        assert_eq!(done["view"]["turn"], "BLACK");
        let queen = done["view"]["pieces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["position"] == "0,3")
            .unwrap();
        assert_eq!(queen["code"], "QUEEN");
        let log = world.log(&id).await;
        assert_eq!(
            log.last().unwrap().kind,
            modern_chess_game::CommandKind::Choose {
                place: None,
                code: Some("QUEEN".to_string()),
                hand: None,
                action: None
            }
        );
    })
    .await;
}

#[tokio::test]
async fn a_win_marks_the_instance_finished_with_the_outcome() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started_with("win", WIN).await;
        world.send(&id, &white, START, at("0,0")).await;

        let (status, done) = world.send(&id, &white, START + 1, at("0,1")).await;

        assert_eq!(status, StatusCode::OK, "{done}");
        assert_eq!(done["status"], "finished");
        assert_eq!(done["outcome"], json!({ "type": "won", "seat": "WHITE" }));
        let stored: (String, Option<DateTime<Utc>>) =
            sqlx::query_as("SELECT status, finished_at FROM game_instances WHERE id = $1")
                .bind(id.parse::<uuid::Uuid>().unwrap())
                .fetch_one(&world.pool)
                .await
                .unwrap();
        assert_eq!(stored.0, "FINISHED");
        assert!(stored.1.is_some());
    })
    .await;
}

#[tokio::test]
async fn a_checkmate_in_chess_finishes_the_game() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, black) = world.started("chess").await;
        // Fool's mate: 1. f3 e5 2. g4 Qh4#
        let moves = [
            (&white, "5,1", "5,2"),
            (&black, "4,6", "4,4"),
            (&white, "6,1", "6,3"),
            (&black, "3,7", "7,3"),
        ];
        let mut revision = START;
        let mut last = Value::Null;
        for (player, from, to) in moves {
            let (status, _) = world.send(&id, player, revision, at(from)).await;
            assert_eq!(status, StatusCode::OK);
            let (status, body) = world.send(&id, player, revision + 1, at(to)).await;
            assert_eq!(status, StatusCode::OK, "{body}");
            revision += 2;
            last = body;
        }

        assert_eq!(last["status"], "finished");
        assert_eq!(last["outcome"], json!({ "type": "won", "seat": "BLACK" }));
    })
    .await;
}

#[tokio::test]
async fn a_draw_marks_it_finished_as_draw() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, first, second) = world.started_with("draw", DRAW).await;
        let mover = match world.snapshot(&id, None).await["view"]["turn"].as_str() {
            Some("WHITE") => first,
            _ => second,
        };
        world.send(&id, &mover, START, at("2,4")).await;

        let (status, done) = world.send(&id, &mover, START + 1, at("2,6")).await;

        assert_eq!(status, StatusCode::OK, "{done}");
        assert_eq!(done["status"], "finished");
        assert_eq!(done["outcome"], json!({ "type": "draw" }));
    })
    .await;
}

#[tokio::test]
async fn a_finished_instance_refuses_commands() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, black) = world.started_with("win", WIN).await;
        world.send(&id, &white, START, at("0,0")).await;
        world.send(&id, &white, START + 1, at("0,1")).await;
        let before = world.snapshot(&id, None).await;

        for (player, revision) in [(&white, START + 2), (&black, START + 2), (&white, START)] {
            let (status, body) = world.send(&id, player, revision, at("0,1")).await;
            assert_eq!(status, StatusCode::CONFLICT, "{body}");
            assert_eq!(body, json!({ "error": "the match is over" }));
        }
        assert_eq!(world.snapshot(&id, None).await, before);
        assert_eq!(world.log(&id).await.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn a_lobby_refuses_commands() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let alice = world.player("Alice");
        let (_, created) = world
            .call(
                Method::POST,
                "/catalog/chess/lobbies",
                Some(&alice.token),
                None,
            )
            .await;
        let id = created["id"].as_str().unwrap().to_string();

        let (status, body) = world.send(&id, &alice, START, at("4,1")).await;

        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body, json!({ "error": "the match has not started" }));
        assert!(world.log(&id).await.is_empty());
    })
    .await;
}

fn resign() -> Value {
    json!({ "type": "resign" })
}

#[tokio::test]
async fn the_seat_to_move_may_resign_and_the_other_seat_wins() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;

        let (status, done) = world.send(&id, &white, START, resign()).await;

        assert_eq!(status, StatusCode::OK, "{done}");
        assert_eq!(done["status"], "finished");
        assert_eq!(
            done["outcome"],
            json!({ "type": "won", "seat": "BLACK", "reason": "resigned" })
        );
        assert_eq!(done["revision"], START + 1);
        let stored: (String, Option<DateTime<Utc>>) =
            sqlx::query_as("SELECT status, finished_at FROM game_instances WHERE id = $1")
                .bind(id.parse::<uuid::Uuid>().unwrap())
                .fetch_one(&world.pool)
                .await
                .unwrap();
        assert_eq!(stored.0, "FINISHED");
        assert!(stored.1.is_some());
        let log = world.log(&id).await;
        assert_eq!(log.len(), 1);
        assert!(log[0].accepted);
        assert_eq!(log[0].kind, modern_chess_game::CommandKind::Resign);
    })
    .await;
}

#[tokio::test]
async fn the_seat_not_to_move_may_resign_too() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, _, black) = world.started("chess").await;

        let (status, done) = world.send(&id, &black, START, resign()).await;

        assert_eq!(status, StatusCode::OK, "{done}");
        assert_eq!(
            done["outcome"],
            json!({ "type": "won", "seat": "WHITE", "reason": "resigned" })
        );
    })
    .await;
}

#[tokio::test]
async fn resigning_does_not_depend_on_the_position_the_player_was_looking_at() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, black) = world.started("chess").await;
        // White moves while black's resignation is on its way, so black's revision is stale.
        play(&world, &id, &white, "4,1", "4,3").await;

        let (status, done) = world.send(&id, &black, START, resign()).await;

        assert_eq!(status, StatusCode::OK, "{done}");
        assert_eq!(done["status"], "finished");
        assert_eq!(
            done["outcome"],
            json!({ "type": "won", "seat": "WHITE", "reason": "resigned" })
        );
    })
    .await;
}

#[tokio::test]
async fn the_opponent_sees_how_the_match_ended() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, black) = world.started("chess").await;
        world.send(&id, &white, START, resign()).await;

        let seen = world.snapshot(&id, Some(&black.token)).await;

        assert_eq!(seen["you"], "BLACK");
        assert_eq!(
            seen["outcome"],
            json!({ "type": "won", "seat": "BLACK", "reason": "resigned" })
        );
    })
    .await;
}

#[tokio::test]
async fn only_a_seated_player_may_resign_a_match_being_played() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, _, _) = world.started("chess").await;
        let stranger = world.player("Carol");
        let uri = format!("/instances/{id}/commands");

        let (status, answer) = world.send(&id, &stranger, START, resign()).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
        assert_eq!(
            answer,
            json!({ "error": "you are not playing in this match" })
        );
        let body = Some(json!({ "revision": START, "kind": resign() }));
        let (status, _) = world.call(Method::POST, &uri, None, body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let after = world.snapshot(&id, None).await;
        assert_eq!(after["status"], "active");
        assert_eq!(after["outcome"], Value::Null);
        assert!(world.log(&id).await.is_empty());
    })
    .await;
}

#[tokio::test]
async fn a_lobby_and_a_finished_match_cannot_be_resigned() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let alice = world.player("Alice");
        let (_, created) = world
            .call(
                Method::POST,
                "/catalog/chess/lobbies",
                Some(&alice.token),
                None,
            )
            .await;
        let lobby = created["id"].as_str().unwrap().to_string();

        let (status, body) = world.send(&lobby, &alice, START, resign()).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body, json!({ "error": "the match has not started" }));

        let (id, white, black) = world.started("chess").await;
        world.send(&id, &white, START, resign()).await;
        let before = world.snapshot(&id, None).await;
        for player in [&white, &black] {
            let (status, body) = world.send(&id, player, START + 1, resign()).await;
            assert_eq!(status, StatusCode::CONFLICT, "{body}");
            assert_eq!(body, json!({ "error": "the match is over" }));
        }
        assert_eq!(world.snapshot(&id, None).await, before);
        assert_eq!(world.log(&id).await.len(), 1);
    })
    .await;
}

#[tokio::test]
async fn only_the_people_seated_may_command() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;
        let stranger = world.player("Carol");
        let uri = format!("/instances/{id}/commands");
        let body = Some(json!({ "revision": START, "kind": at("4,1") }));

        let (status, answer) = world.send(&id, &stranger, START, at("4,1")).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
        assert_eq!(
            answer,
            json!({ "error": "you are not playing in this match" })
        );

        for token in [None, Some("garbage")] {
            let (status, answer) = world.call(Method::POST, &uri, token, body.clone()).await;
            assert_eq!(status, StatusCode::UNAUTHORIZED);
            assert_eq!(answer, json!({ "error": "sign in required" }));
        }

        let missing = uuid::Uuid::new_v4();
        let (status, _) = world.send(&missing.to_string(), &white, 0, at("4,1")).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = world.send("not-an-id", &white, 0, at("4,1")).await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        for broken in [
            json!({}),
            json!({ "revision": -1, "kind": at("4,1") }),
            json!({ "revision": 0, "kind": { "type": "teleport" } }),
            json!({ "revision": 0, "kind": { "type": "select", "position": "4,1" } }),
            json!({ "revision": 0, "kind": { "type": "choose", "x": 4, "y": 1 } }),
            json!({ "revision": 0, "kind": { "type": "choose", "place": "a".repeat(65) } }),
        ] {
            let (status, _) = world
                .call(Method::POST, &uri, Some(&white.token), Some(broken))
                .await;
            assert_eq!(status, StatusCode::BAD_REQUEST);
        }
        assert!(world.log(&id).await.is_empty());
    })
    .await;
}

#[tokio::test]
async fn a_square_not_on_the_board_is_refused_and_logged() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;

        for name in ["9,9".to_string(), "a".repeat(64)] {
            let (status, body) = world.send(&id, &white, START, at(&name)).await;

            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
            assert_eq!(body, json!({ "error": "that is not a legal move" }));
        }

        let log = world.log(&id).await;
        assert_eq!(log.len(), 2);
        assert!(log.iter().all(|line| !line.accepted));
        assert_eq!(world.snapshot(&id, None).await["revision"], START);
    })
    .await;
}

/// Sends a command at the match's current revision.
async fn act(world: &World, id: &str, by: &Player, kind: Value) -> (StatusCode, Value) {
    let revision = world.snapshot(id, None).await["revision"].as_u64().unwrap();
    world.send(id, by, revision, kind).await
}

/// Drops a piece from the hand: two answers, the hand piece and then the square. Returns the
/// answer to the second, or to the first when that was refused.
async fn drop_piece(
    world: &World,
    id: &str,
    by: &Player,
    name: &str,
    square: &str,
) -> (StatusCode, Value) {
    let (status, body) = act(world, id, by, hand(name)).await;
    if status != StatusCode::OK {
        return (status, body);
    }
    act(world, id, by, at(square)).await
}

/// Picks up the piece on `from` and moves it to `to`.
async fn play(world: &World, id: &str, by: &Player, from: &str, to: &str) {
    let (status, body) = act(world, id, by, at(from)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = act(world, id, by, at(to)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// A Shogi match in which SENTE has just taken GOTE's bishop, and GOTE has answered.
async fn shogi_after_a_bishop_trade(world: &World) -> (String, Player, Player) {
    let (id, sente, gote) = world.started("shogi").await;
    play(world, &id, &sente, "6,2", "6,3").await;
    play(world, &id, &gote, "2,6", "2,5").await;
    play(world, &id, &sente, "7,1", "1,7").await;
    let (status, body) = act(world, &id, &sente, code("HORSE")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    play(world, &id, &gote, "0,6", "0,5").await;
    (id, sente, gote)
}

#[tokio::test]
async fn a_drop_is_two_choose_commands() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, sente, gote) = shogi_after_a_bishop_trade(&world).await;

        let seen = world.snapshot(&id, Some(&sente.token)).await;
        assert_eq!(seen["view"]["hand"]["SENTE"], json!({ "BISHOP": 1 }));
        assert_eq!(seen["view"]["hand"]["GOTE"], json!({}));
        assert_eq!(seen["view"]["choice"]["hand"], json!(["BISHOP"]));
        let before = seen["revision"].as_u64().unwrap();

        // The hand piece first: the match waits for its square.
        let (status, body) = world.send(&id, &sente, before, hand("BISHOP")).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["revision"], before + 1);
        assert_eq!(body["view"]["turn"], "SENTE");
        assert_eq!(body["view"]["hand"]["SENTE"], json!({ "BISHOP": 1 }));
        let seen = world.snapshot(&id, Some(&sente.token)).await;
        assert_eq!(
            seen["view"]["choice"]["chosen"],
            json!([{ "hand": "BISHOP" }])
        );
        assert!(seen["view"]["choice"]["places"]
            .as_array()
            .unwrap()
            .contains(&json!({ "name": "4,4", "for": "PLACE" })));

        let (status, body) = world.send(&id, &sente, before + 1, at("4,4")).await;

        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["revision"], before + 2);
        assert_eq!(body["view"]["hand"]["SENTE"], json!({}));
        assert_eq!(
            body["view"]["choice"]["places"],
            json!([]),
            "GOTE is to move"
        );
        assert_eq!(body["view"]["turn"], "GOTE");
        let bishop = body["view"]["pieces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["position"] == "4,4")
            .unwrap();
        assert_eq!(
            (bishop["code"].as_str(), bishop["player"].as_str()),
            (Some("BISHOP"), Some("SENTE"))
        );
        let log = world.log(&id).await;
        let kinds: Vec<_> = log[log.len() - 2..].iter().map(|line| &line.kind).collect();
        assert_eq!(
            kinds,
            [
                &modern_chess_game::CommandKind::Choose {
                    place: None,
                    code: None,
                    hand: Some("BISHOP".to_string()),
                    action: None
                },
                &modern_chess_game::CommandKind::Choose {
                    place: Some("4,4".to_string()),
                    code: None,
                    hand: None,
                    action: None
                },
            ]
        );
        assert!(log[log.len() - 2..].iter().all(|line| line.accepted));

        // The match was stored whole: GOTE can carry on from a fresh read.
        play(&world, &id, &gote, "4,6", "4,5").await;
    })
    .await;
}

#[tokio::test]
async fn an_illegal_drop_is_422_logged_and_changes_nothing() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, sente, _) = shogi_after_a_bishop_trade(&world).await;
        let before = world.snapshot(&id, None).await;

        // A piece that is not in the hand is refused at the first answer.
        let (status, body) = drop_piece(&world, &id, &sente, "ROOK", "4,4").await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        let message = body["error"].as_str().unwrap();
        assert!(!message.is_empty() && !message.contains('{'), "{message}");
        let after = world.snapshot(&id, None).await;
        assert_eq!(after["revision"], before["revision"]);
        assert_eq!(after["view"], before["view"]);

        // A square off the board is refused at the second, and the hand piece stays chosen.
        let (status, body) = drop_piece(&world, &id, &sente, "BISHOP", "40,40").await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        let after = world.snapshot(&id, Some(&sente.token)).await;
        assert_eq!(
            after["revision"].as_u64(),
            Some(before["revision"].as_u64().unwrap() + 1)
        );
        assert_eq!(
            after["view"]["choice"]["chosen"],
            json!([{ "hand": "BISHOP" }])
        );
        let log = world.log(&id).await;
        assert!(!log[log.len() - 1].accepted);
        assert!(!log[log.len() - 3].accepted);
    })
    .await;
}

#[tokio::test]
async fn only_the_seat_to_move_may_drop() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, _, gote) = shogi_after_a_bishop_trade(&world).await;

        let (status, body) = act(&world, &id, &gote, hand("BISHOP")).await;

        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body, json!({ "error": "it is not your turn" }));
    })
    .await;
}

#[tokio::test]
async fn a_drop_in_a_game_without_a_hand_is_422() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;

        let (status, body) = world.send(&id, &white, START, hand("PAWN")).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        assert_eq!(world.snapshot(&id, None).await["revision"], START);
        assert!(!world.log(&id).await[0].accepted);
    })
    .await;
}

#[tokio::test]
async fn polling_with_the_current_revision_says_nothing_changed() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;
        world.send(&id, &white, START, at("4,1")).await;

        let (status, same) = world
            .call(
                Method::GET,
                &format!("/instances/{id}?since={}", START + 1),
                None,
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(same, json!({ "changed": false }));

        let (_, older) = world
            .call(
                Method::GET,
                &format!("/instances/{id}?since={START}"),
                Some(&white.token),
                None,
            )
            .await;
        assert_eq!(older["revision"], START + 1);
        assert_eq!(older["view"]["choice"]["start"], json!("4,1"));

        let (status, _) = world
            .call(
                Method::GET,
                &format!("/instances/{id}?since=soon"),
                None,
                None,
            )
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    })
    .await;
}

#[tokio::test]
async fn a_snapshot_tells_each_viewer_their_seat() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, black) = world.started("chess").await;
        let stranger = world.player("Carol");
        let expired = world.keys.token_for(
            white.id,
            "Alice",
            (Utc::now() - Duration::hours(1)).timestamp(),
        );

        assert_eq!(
            world.snapshot(&id, Some(&white.token)).await["you"],
            "WHITE"
        );
        assert_eq!(
            world.snapshot(&id, Some(&black.token)).await["you"],
            "BLACK"
        );
        assert_eq!(
            world.snapshot(&id, Some(&stranger.token)).await["you"],
            Value::Null
        );
        assert_eq!(world.snapshot(&id, None).await["you"], Value::Null);
        // Watching needs no sign-in, so a bad token is simply "nobody".
        assert_eq!(
            world.snapshot(&id, Some(&expired)).await["you"],
            Value::Null
        );
        assert_eq!(
            world.snapshot(&id, None).await["players"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    })
    .await;
}

fn pass() -> Value {
    json!({ "type": "choose", "action": "PASS" })
}

#[tokio::test]
async fn a_stone_can_be_placed_from_the_supply_and_two_passes_end_the_game() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, black, white) = world.started("go").await;

        let (status, body) = act(&world, &id, &black, at("4,4")).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["view"]["turn"], "WHITE");
        let seen = world.snapshot(&id, Some(&white.token)).await;
        assert!(seen["view"]["choice"]["actions"]
            .as_array()
            .unwrap()
            .contains(&json!({ "name": "PASS", "plays": true })));

        let (status, body) = act(&world, &id, &white, at("0,0")).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = act(&world, &id, &black, pass()).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["view"]["passes"], 1);
        let (status, body) = act(&world, &id, &white, pass()).await;
        assert_eq!(status, StatusCode::OK, "{body}");

        // One stone each and a single empty region touching both: 1 point against 1 + 7.5.
        assert_eq!(body["status"], "finished");
        assert_eq!(body["outcome"], json!({ "type": "won", "seat": "WHITE" }));
        let log = world.log(&id).await;
        assert_eq!(log.iter().filter(|line| line.accepted).count(), 4);
    })
    .await;
}

#[tokio::test]
async fn passing_in_a_game_that_does_not_allow_it_is_422_and_logged() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;

        let (status, body) = world.send(&id, &white, START, pass()).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        let log = world.log(&id).await;
        assert!(log.last().is_some_and(|line| !line.accepted));
    })
    .await;
}

#[tokio::test]
async fn the_choice_is_hidden_from_the_player_not_to_move() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, black) = world.started("chess").await;
        world.send(&id, &white, START, at("4,1")).await;

        let own = world.snapshot(&id, Some(&white.token)).await;
        assert_eq!(own["view"]["choice"]["chosen"], json!([{ "place": "4,1" }]));
        assert_eq!(own["view"]["choice"]["start"], json!("4,1"));
        assert_eq!(
            own["view"]["choice"]["places"],
            json!([{ "name": "4,2", "for": "MOVE" }, { "name": "4,3", "for": "MOVE" }])
        );

        // The opponent and anyone watching see the board, but not what is being weighed.
        let empty = json!({
            "chosen": [], "start": null, "places": [], "codes": [], "hand": [], "actions": []
        });
        for (who, token) in [("the opponent", Some(&black.token)), ("a spectator", None)] {
            let seen = world.snapshot(&id, token.map(String::as_str)).await;
            assert_eq!(seen["view"]["choice"], empty, "{who}");
            assert_eq!(seen["view"]["pieces"], own["view"]["pieces"], "{who}");
            assert_eq!(seen["view"]["turn"], "WHITE", "{who}");
        }
    })
    .await;
}

#[tokio::test]
async fn the_choice_stays_hidden_during_a_promotion() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, black) = world.started_with("promote", PROMOTE).await;
        play(&world, &id, &white, "0,2", "0,3").await;

        let own = world.snapshot(&id, Some(&white.token)).await;
        assert_eq!(own["view"]["choice"]["codes"], json!(["QUEEN", "ROOK"]));

        for (who, token) in [("the opponent", Some(&black.token)), ("a spectator", None)] {
            let seen = world.snapshot(&id, token.map(String::as_str)).await;
            assert_eq!(seen["view"]["choice"]["codes"], json!([]), "{who}");
            assert_eq!(seen["view"]["choice"]["chosen"], json!([]), "{who}");
            // The pawn already stands on the last rank for everyone.
            assert_eq!(seen["view"]["pieces"], own["view"]["pieces"], "{who}");
        }
    })
    .await;
}

#[tokio::test]
async fn only_the_player_to_move_is_told_where_a_stone_may_go() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, black, white) = world.started("go").await;

        let own = world.snapshot(&id, Some(&black.token)).await;
        assert!(!own["view"]["choice"]["places"]
            .as_array()
            .unwrap()
            .is_empty());

        for (who, token) in [("the opponent", Some(&white.token)), ("a spectator", None)] {
            let seen = world.snapshot(&id, token.map(String::as_str)).await;
            assert_eq!(seen["view"]["choice"]["places"], json!([]), "{who}");
            assert_eq!(seen["view"]["choice"]["actions"], json!([]), "{who}");
        }
    })
    .await;
}

#[tokio::test]
async fn a_choose_command_needs_exactly_one_answer() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;

        for kind in [
            json!({ "type": "choose" }),
            json!({ "type": "choose", "place": null }),
            json!({ "type": "choose", "place": "4,1", "code": "QUEEN" }),
            json!({ "type": "choose", "hand": "PAWN", "place": "4,4" }),
            json!({ "type": "choose", "place": "4,1", "code": "Q", "hand": "P", "action": "A" }),
        ] {
            let (status, body) = world.send(&id, &white, START, kind.clone()).await;

            assert_eq!(status, StatusCode::BAD_REQUEST, "{kind}: {body}");
        }

        assert!(world.log(&id).await.is_empty(), "refused before logging");
        assert_eq!(world.snapshot(&id, None).await["revision"], START);
    })
    .await;
}

#[tokio::test]
async fn every_answer_string_is_limited_to_64_bytes() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;

        for key in ["place", "code", "hand", "action"] {
            let mut kind = json!({ "type": "choose" });
            kind[key] = json!("a".repeat(65));

            let (status, body) = world.send(&id, &white, START, kind.clone()).await;

            assert_eq!(status, StatusCode::BAD_REQUEST, "{kind}: {body}");
        }
        assert!(world.log(&id).await.is_empty());

        // 64 bytes is allowed through to the engine, which refuses what it does not know.
        let mut kind = json!({ "type": "choose" });
        kind["code"] = json!("a".repeat(64));
        let (status, _) = world.send(&id, &white, START, kind).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    })
    .await;
}

#[tokio::test]
async fn cancel_is_logged_and_clears_the_choice() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;
        world.send(&id, &white, START, at("4,1")).await;

        let (status, body) = world.send(&id, &white, START + 1, cancel()).await;

        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["revision"], START + 2);
        assert_eq!(body["view"]["choice"]["chosen"], json!([]));
        assert_eq!(body["view"]["choice"]["start"], Value::Null);
        assert_eq!(body["view"]["turn"], "WHITE");
        let log = world.log(&id).await;
        assert_eq!(
            log.last().unwrap().kind,
            modern_chess_game::CommandKind::Cancel
        );
        assert!(log.last().unwrap().accepted);
    })
    .await;
}

#[tokio::test]
async fn a_promotion_cannot_be_cancelled() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started_with("promote", PROMOTE).await;
        play(&world, &id, &white, "0,2", "0,3").await;

        let (status, body) = act(&world, &id, &white, cancel()).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        let seen = world.snapshot(&id, Some(&white.token)).await;
        assert_eq!(seen["view"]["choice"]["codes"], json!(["QUEEN", "ROOK"]));
    })
    .await;
}
