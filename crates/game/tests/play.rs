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

fn select(x: u8, y: u8) -> Value {
    json!({ "type": "select", "x": x, "y": y })
}

fn mv(x: u8, y: u8) -> Value {
    json!({ "type": "move", "x": x, "y": y })
}

#[tokio::test]
async fn only_the_seat_to_move_may_select() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, black) = world.started("chess").await;

        let (status, body) = world.send(&id, &black, START, select(4, 6)).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
        assert_eq!(body, json!({ "error": "it is not your turn" }));
        assert_eq!(world.snapshot(&id, None).await["revision"], START);
        assert!(world.log(&id).await.is_empty());

        let (status, body) = world.send(&id, &white, START, select(4, 1)).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["view"]["selected"], json!([4, 1]));
    })
    .await;
}

#[tokio::test]
async fn move_advances_the_turn_and_bumps_the_revision() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, black) = world.started("chess").await;

        let (_, picked) = world.send(&id, &white, START, select(4, 1)).await;
        assert_eq!(picked["revision"], START + 1);
        assert_eq!(picked["view"]["destinations"], json!([[4, 2], [4, 3]]));

        let (status, moved) = world.send(&id, &white, START + 1, mv(4, 3)).await;
        assert_eq!(status, StatusCode::OK, "{moved}");
        assert_eq!(moved["revision"], START + 2);
        assert_eq!(moved["status"], "active");
        assert_eq!(moved["view"]["turn"], "BLACK");
        assert_eq!(moved["you"], "WHITE");

        // The opponent sees the same position, and now it is their turn.
        let seen = world.snapshot(&id, Some(&black.token)).await;
        assert_eq!(seen["view"], moved["view"]);
        assert_eq!(seen["you"], "BLACK");
        let (status, _) = world.send(&id, &black, START + 2, select(4, 6)).await;
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
            let (status, body) = world.send(&id, &white, revision, select(4, 1)).await;
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
        world.send(&id, &white, START, select(4, 1)).await;
        world.send(&id, &white, START + 1, mv(4, 3)).await;

        // A double click: the same move, still based on the revision before it.
        let (status, _) = world.send(&id, &white, START + 1, mv(4, 3)).await;

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
                    Some(json!({ "revision": START, "kind": select(4, 1) })),
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
            Some(json!({ "revision": START, "kind": select(4, 1) })),
        )
        .await;

        assert_eq!(status, StatusCode::OK, "{body}");
    })
    .await;
}

#[tokio::test]
async fn an_illegal_destination_is_422_and_logged_as_rejected() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;
        world.send(&id, &white, START, select(4, 1)).await;

        let (status, body) = world.send(&id, &white, START + 1, mv(4, 5)).await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
        let message = body["error"].as_str().unwrap();
        assert!(!message.is_empty() && !message.contains('{'), "{message}");
        let seen = world.snapshot(&id, None).await;
        assert_eq!(
            seen["revision"],
            START + 1,
            "a refused move changes nothing"
        );
        assert_eq!(seen["view"]["selected"], json!([4, 1]));
        let log = world.log(&id).await;
        assert_eq!(log.len(), 2);
        assert!(log[0].accepted);
        assert!(!log[1].accepted);
        assert_eq!(log[1].actor, white.id);
        assert!(log[1].error.is_some());

        // Picking up an empty square is refused the same way.
        let (status, _) = world.send(&id, &white, START + 1, select(4, 4)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(world.log(&id).await.len(), 3);
    })
    .await;
}

#[tokio::test]
async fn promotion_asks_then_applies() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started_with("promote", PROMOTE).await;

        world.send(&id, &white, START, select(0, 2)).await;
        let (_, asked) = world.send(&id, &white, START + 1, mv(0, 3)).await;
        assert_eq!(asked["view"]["promotion"], json!(["QUEEN", "ROOK"]));
        assert_eq!(asked["view"]["turn"], "WHITE");

        let (status, done) = world
            .send(
                &id,
                &white,
                START + 2,
                json!({ "type": "promote", "code": "QUEEN" }),
            )
            .await;

        assert_eq!(status, StatusCode::OK, "{done}");
        assert_eq!(done["view"]["promotion"], Value::Null);
        assert_eq!(done["view"]["turn"], "BLACK");
        let queen = done["view"]["pieces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["x"] == 0 && p["y"] == 3)
            .unwrap();
        assert_eq!(queen["code"], "QUEEN");
    })
    .await;
}

#[tokio::test]
async fn a_win_marks_the_instance_finished_with_the_outcome() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started_with("win", WIN).await;
        world.send(&id, &white, START, select(0, 0)).await;

        let (status, done) = world.send(&id, &white, START + 1, mv(0, 1)).await;

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
            (&white, (5, 1), (5, 2)),
            (&black, (4, 6), (4, 4)),
            (&white, (6, 1), (6, 3)),
            (&black, (3, 7), (7, 3)),
        ];
        let mut revision = START;
        let mut last = Value::Null;
        for (player, from, to) in moves {
            let (status, _) = world
                .send(&id, player, revision, select(from.0, from.1))
                .await;
            assert_eq!(status, StatusCode::OK);
            let (status, body) = world.send(&id, player, revision + 1, mv(to.0, to.1)).await;
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
        world.send(&id, &mover, START, select(2, 4)).await;

        let (status, done) = world.send(&id, &mover, START + 1, mv(2, 6)).await;

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
        world.send(&id, &white, START, select(0, 0)).await;
        world.send(&id, &white, START + 1, mv(0, 1)).await;
        let before = world.snapshot(&id, None).await;

        for (player, revision) in [(&white, START + 2), (&black, START + 2), (&white, START)] {
            let (status, body) = world.send(&id, player, revision, select(0, 1)).await;
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

        let (status, body) = world.send(&id, &alice, START, select(4, 1)).await;

        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body, json!({ "error": "the match has not started" }));
        assert!(world.log(&id).await.is_empty());
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
        let body = Some(json!({ "revision": START, "kind": select(4, 1) }));

        let (status, answer) = world.send(&id, &stranger, START, select(4, 1)).await;
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
        let (status, _) = world
            .send(&missing.to_string(), &white, 0, select(4, 1))
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = world.send("not-an-id", &white, 0, select(4, 1)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        for broken in [
            json!({}),
            json!({ "revision": -1, "kind": select(4, 1) }),
            json!({ "revision": 0, "kind": { "type": "teleport" } }),
            json!({ "revision": 0, "kind": { "type": "select", "x": 300, "y": 1 } }),
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
async fn drops_are_refused_because_the_session_does_not_play_them_yet() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;

        let (status, _) = world
            .send(
                &id,
                &white,
                START,
                json!({ "type": "drop", "code": "PAWN", "x": 0, "y": 3 }),
            )
            .await;

        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    })
    .await;
}

#[tokio::test]
async fn polling_with_the_current_revision_says_nothing_changed() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (id, white, _) = world.started("chess").await;
        world.send(&id, &white, START, select(4, 1)).await;

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
                None,
                None,
            )
            .await;
        assert_eq!(older["revision"], START + 1);
        assert_eq!(older["view"]["selected"], json!([4, 1]));

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
