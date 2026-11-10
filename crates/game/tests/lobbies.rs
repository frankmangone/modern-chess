//! Creating and joining lobbies over HTTP, against a real Postgres (`make db-up`).
//!
//! Nothing here knows the social module exists: tokens come from throwaway keys, and the
//! game router is configured with the public half only, exactly as game-api is.

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use chrono::{DateTime, Duration, TimeZone, Utc};
use modern_chess_game::seed;
use modern_chess_game::store::{InstanceRepo, PageRepo, SpecRepo};
use modern_chess_game::{router, GameInstance, GamePage, GameState, PageSection, Slug};
use modern_chess_kernel::{TokenVerifier, UserId};
use modern_chess_testdb::with_db;
use modern_chess_testkeys::TestKeys;
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::task::JoinSet;
use tower::ServiceExt;

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap()
}

struct World {
    app: Router,
    keys: TestKeys,
    pool: PgPool,
}

/// A migrated database with the launch games seeded, plus a page for the four-seat game.
async fn world(pool: PgPool) -> World {
    modern_chess_game::migrate(&pool).await.unwrap();
    let specs = SpecRepo::new(pool.clone());
    let pages = PageRepo::new(pool.clone());
    seed::run(&specs, &pages, t0()).await.unwrap();
    pages
        .upsert(&GamePage {
            name: None,
            preview_variant: None,
            slug: Slug::parse("4player-chess").unwrap(),
            tagline: "Four armies".to_string(),
            description: "d".to_string(),
            tags: vec![],
            accent: None,
            cover_image: None,
            skin: None,
            sections: vec![PageSection::OpenLobbies],
            featured: false,
            sort_order: 9,
        })
        .await
        .unwrap();

    let keys = TestKeys::generate();
    let verifier = TokenVerifier::from_public_pem(&keys.public_pem).unwrap();
    World {
        app: router(GameState::new(pool.clone(), verifier)),
        keys,
        pool,
    }
}

struct Player {
    token: String,
}

impl World {
    fn player(&self, handle: &str) -> Player {
        let id = UserId::new();
        let token = self
            .keys
            .token_for(id, handle, (Utc::now() + Duration::hours(1)).timestamp());
        Player { token }
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

    async fn create_lobby(&self, slug: &str, by: &Player) -> String {
        let (status, body) = self
            .call(
                Method::POST,
                &format!("/catalog/{slug}/lobbies"),
                Some(&by.token),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_str().unwrap().to_string()
    }

    async fn join(&self, id: &str, by: &Player, seat: &str) -> (StatusCode, Value) {
        self.call(
            Method::POST,
            &format!("/instances/{id}/join"),
            Some(&by.token),
            Some(json!({ "seat": seat })),
        )
        .await
    }

    async fn lobby(&self, id: &str) -> Value {
        let (status, body) = self
            .call(Method::GET, &format!("/instances/{id}"), None, None)
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    }

    async fn open_lobbies(&self, slug: &str) -> Vec<Value> {
        let (status, detail) = self
            .call(Method::GET, &format!("/catalog/{slug}"), None, None)
            .await;
        assert_eq!(status, StatusCode::OK);
        detail["open_lobbies"].as_array().unwrap().clone()
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

fn handles(lobby: &Value) -> Vec<(String, String)> {
    lobby["players"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["seat"].as_str().unwrap().to_string(),
                p["handle"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[tokio::test]
async fn creating_a_lobby_requires_a_valid_token() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let expired = world.keys.token_for(
            UserId::new(),
            "Alice",
            (Utc::now() - Duration::hours(1)).timestamp(),
        );
        let foreign = TestKeys::generate().token_for(
            UserId::new(),
            "Alice",
            (Utc::now() + Duration::hours(1)).timestamp(),
        );

        for token in [
            None,
            Some("garbage"),
            Some(expired.as_str()),
            Some(foreign.as_str()),
        ] {
            let (status, body) = world
                .call(Method::POST, "/catalog/chess/lobbies", token, None)
                .await;
            assert_eq!(status, StatusCode::UNAUTHORIZED, "{token:?}");
            assert_eq!(body, json!({ "error": "sign in required" }), "{token:?}");
        }

        let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM game_instances")
            .fetch_one(&world.pool)
            .await
            .unwrap();
        assert_eq!(rows, 0, "nothing was created");
    })
    .await;
}

#[tokio::test]
async fn creating_a_lobby_for_an_unknown_game_is_404() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let alice = world.player("Alice");

        let (status, body) = world
            .call(
                Method::POST,
                "/catalog/nope/lobbies",
                Some(&alice.token),
                None,
            )
            .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body, json!({ "error": "game not found" }));
    })
    .await;
}

#[tokio::test]
async fn creator_is_seated_in_the_first_free_seat_with_the_handle_from_the_token() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let alice = world.player("Alice");

        let id = world.create_lobby("chess", &alice).await;
        let lobby = world.lobby(&id).await;

        assert_eq!(lobby["status"], "lobby");
        assert_eq!(lobby["slug"], "chess");
        assert_eq!(lobby["capacity"], 2);
        assert_eq!(lobby["seats"], json!(["WHITE", "BLACK"]));
        assert_eq!(
            handles(&lobby),
            [("WHITE".to_string(), "Alice".to_string())]
        );
        assert!(lobby["started_at"].is_null());
    })
    .await;
}

#[tokio::test]
async fn second_player_joining_a_two_seat_game_makes_it_active() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (alice, bob) = (world.player("Alice"), world.player("Bobby"));
        let id = world.create_lobby("chess", &alice).await;

        let (status, joined) = world.join(&id, &bob, "BLACK").await;

        assert_eq!(status, StatusCode::OK, "{joined}");
        assert_eq!(joined["status"], "active");
        assert!(joined["started_at"].is_string());
        assert_eq!(
            handles(&joined),
            [
                ("WHITE".to_string(), "Alice".to_string()),
                ("BLACK".to_string(), "Bobby".to_string())
            ]
        );
        assert!(
            world.open_lobbies("chess").await.is_empty(),
            "a started game is not an open lobby"
        );
    })
    .await;
}

#[tokio::test]
async fn every_join_changes_the_revision_so_a_waiting_player_sees_it() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (alice, bob) = (world.player("Alice"), world.player("Bobby"));
        let id = world.create_lobby("chess", &alice).await;
        let before = world.lobby(&id).await["revision"].as_u64().unwrap();

        let (status, joined) = world.join(&id, &bob, "BLACK").await;

        assert_eq!(status, StatusCode::OK, "{joined}");
        let after = joined["revision"].as_u64().unwrap();
        assert!(after > before, "revision stayed at {before}");
        let (_, asked) = world
            .call(
                Method::GET,
                &format!("/instances/{id}?since={before}"),
                Some(&alice.token),
                None,
            )
            .await;
        assert_ne!(
            asked["changed"],
            json!(false),
            "a player polling from revision {before} was told nothing happened"
        );
        assert_eq!(asked["status"], "active");
        assert_eq!(asked["revision"].as_u64(), Some(after));
    })
    .await;
}

#[tokio::test]
async fn four_player_lobby_stays_open_until_four() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let players: Vec<Player> = ["Pone", "Ptwo", "Pthree", "Pfour"]
            .into_iter()
            .map(|h| world.player(h))
            .collect();
        let id = world.create_lobby("4player-chess", &players[0]).await;

        for (player, seat) in players[1..3].iter().zip(["BLACK", "SILVER"]) {
            let (status, lobby) = world.join(&id, player, seat).await;
            assert_eq!(status, StatusCode::OK, "{lobby}");
            assert_eq!(lobby["status"], "lobby");
        }
        let open = world.open_lobbies("4player-chess").await;
        assert_eq!(open.len(), 1);
        assert_eq!(open[0]["players"].as_array().unwrap().len(), 3);
        assert_eq!(open[0]["capacity"], 4);

        let (status, lobby) = world.join(&id, &players[3], "GOLD").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(lobby["status"], "active");
        assert!(world.open_lobbies("4player-chess").await.is_empty());
    })
    .await;
}

#[tokio::test]
async fn cannot_join_twice() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let alice = world.player("Alice");
        let id = world.create_lobby("chess", &alice).await;

        let (status, body) = world.join(&id, &alice, "BLACK").await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(
            body,
            json!({ "error": "you already have a seat in this game" })
        );
        assert_eq!(
            world.lobby(&id).await["players"].as_array().unwrap().len(),
            1
        );
    })
    .await;
}

#[tokio::test]
async fn joining_a_taken_seat_is_409() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (alice, bob) = (world.player("Alice"), world.player("Bobby"));
        let id = world.create_lobby("chess", &alice).await;

        let (status, body) = world.join(&id, &bob, "WHITE").await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body, json!({ "error": "that seat is already taken" }));
    })
    .await;
}

#[tokio::test]
async fn joining_a_game_that_already_started_is_409() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (alice, bob, carol) = (
            world.player("Alice"),
            world.player("Bobby"),
            world.player("Carol"),
        );
        let id = world.create_lobby("chess", &alice).await;
        world.join(&id, &bob, "BLACK").await;

        let (status, body) = world.join(&id, &carol, "BLACK").await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(
            body,
            json!({ "error": "this game is no longer open to join" })
        );
    })
    .await;
}

#[tokio::test]
async fn joining_with_bad_input_gets_a_plain_error() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let (alice, bob) = (world.player("Alice"), world.player("Bobby"));
        let id = world.create_lobby("chess", &alice).await;

        let (unknown_seat, body) = world.join(&id, &bob, "PURPLE").await;
        assert_eq!(unknown_seat, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            body,
            json!({ "error": "there is no such seat in this game" })
        );

        let ghost = UserId::new().to_string();
        for lobby in [ghost.as_str(), "not-an-id"] {
            let (status, body) = world.join(lobby, &bob, "BLACK").await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{lobby}");
            assert_eq!(body, json!({ "error": "match not found" }), "{lobby}");
        }

        let (no_seat, body) = world
            .call(
                Method::POST,
                &format!("/instances/{id}/join"),
                Some(&bob.token),
                Some(json!({})),
            )
            .await;
        assert_eq!(no_seat, StatusCode::BAD_REQUEST);
        assert_eq!(body, json!({ "error": "the request could not be read" }));

        let (anonymous, _) = world
            .call(
                Method::POST,
                &format!("/instances/{id}/join"),
                None,
                Some(json!({ "seat": "BLACK" })),
            )
            .await;
        assert_eq!(anonymous, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn viewing_an_unknown_match_is_404() {
    with_db(|pool| async move {
        let world = world(pool).await;

        for id in [UserId::new().to_string(), "not-an-id".to_string()] {
            let (status, body) = world
                .call(Method::GET, &format!("/instances/{id}"), None, None)
                .await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{id}");
            assert_eq!(body, json!({ "error": "match not found" }), "{id}");
        }
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_joins_over_http_seat_one_player() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let alice = world.player("Alice");
        let id = world.create_lobby("chess", &alice).await;

        let mut racers = JoinSet::new();
        for n in 0..6 {
            let player = world.player(&format!("Racer{n}"));
            let app = world.app.clone();
            let uri = format!("/instances/{id}/join");
            racers.spawn(async move {
                call(
                    &app,
                    Method::POST,
                    &uri,
                    Some(&player.token),
                    Some(json!({ "seat": "BLACK" })),
                )
                .await
            });
        }

        let mut winners = 0;
        let mut conflicts = 0;
        while let Some(result) = racers.join_next().await {
            match result.unwrap().0 {
                StatusCode::OK => winners += 1,
                StatusCode::CONFLICT => conflicts += 1,
                other => panic!("unexpected status {other}"),
            }
        }

        assert_eq!(
            (winners, conflicts),
            (1, 5),
            "one wins, the rest see a clear conflict"
        );
        let lobby = world.lobby(&id).await;
        assert_eq!(lobby["players"].as_array().unwrap().len(), 2);
        let black: Vec<_> = handles(&lobby)
            .into_iter()
            .filter(|(seat, _)| seat == "BLACK")
            .collect();
        assert_eq!(black.len(), 1, "never two players in one seat");
    })
    .await;
}

#[tokio::test]
async fn open_lobbies_hide_those_older_than_24_hours() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let spec = SpecRepo::new(world.pool.clone())
            .latest(&Slug::parse("chess").unwrap())
            .await
            .unwrap()
            .unwrap();
        let instances = InstanceRepo::new(world.pool.clone());
        let stale = GameInstance::new_lobby(&spec, Utc::now() - Duration::hours(25)).unwrap();
        let recent = GameInstance::new_lobby(&spec, Utc::now() - Duration::hours(23)).unwrap();
        instances.create(&stale).await.unwrap();
        instances.create(&recent).await.unwrap();
        let alice = world.player("Alice");
        let newest = world.create_lobby("chess", &alice).await;

        let open = world.open_lobbies("chess").await;

        let ids: Vec<_> = open
            .iter()
            .map(|l| l["id"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            ids,
            [newest, recent.id().to_string()],
            "newest first, the stale one hidden"
        );
        assert_eq!(open[0]["capacity"], 2);
        assert_eq!(open[0]["players"][0]["handle"], "Alice");
        assert!(open[0]["created_at"].is_string());
        assert_eq!(
            world.lobby(&stale.id().to_string()).await["status"],
            "lobby",
            "a hidden lobby can still be opened by its link"
        );
    })
    .await;
}

#[tokio::test]
async fn game_api_serves_the_catalogue_and_lobbies_with_social_api_stopped() {
    with_db(|pool| async move {
        // The world holds the public key and nothing else: no social module, no database of
        // users, no way to sign. If it works here, it works with social-api switched off.
        let world = world(pool).await;
        let alice = world.player("Alice");

        let (catalog, _) = world.call(Method::GET, "/catalog", None, None).await;
        let id = world.create_lobby("chess", &alice).await;
        let (joined, _) = world.join(&id, &world.player("Bobby"), "BLACK").await;

        assert_eq!(catalog, StatusCode::OK);
        assert_eq!(joined, StatusCode::OK);
    })
    .await;
}
