//! The catalogue and lobbies over HTTP, for games that contain variants (`make db-up`).

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use chrono::{DateTime, Duration, TimeZone, Utc};
use modern_chess_game::seed;
use modern_chess_game::store::{PageRepo, SpecRepo};
use modern_chess_game::{router, GamePage, GameState, PageSection, Slug, Variant};
use modern_chess_kernel::{TokenVerifier, UserId};
use modern_chess_testdb::with_db;
use modern_chess_testkeys::TestKeys;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

const CHESS: &str = include_str!("../../engine/specs/chess.json");

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).unwrap()
}

fn slug(raw: &str) -> Slug {
    Slug::parse(raw).unwrap()
}

struct World {
    app: Router,
    keys: TestKeys,
    pool: PgPool,
}

/// A migrated database with the launch games seeded from the embedded files.
async fn world(pool: PgPool) -> World {
    modern_chess_game::migrate(&pool).await.unwrap();
    let specs = SpecRepo::new(pool.clone());
    let pages = PageRepo::new(pool.clone());
    let report = seed::run(&specs, &pages, t0()).await.unwrap();
    assert!(report.skipped.is_empty(), "{:?}", report.skipped);
    bare_world(pool)
}

fn bare_world(pool: PgPool) -> World {
    let keys = TestKeys::generate();
    let verifier = TokenVerifier::from_public_pem(&keys.public_pem).unwrap();
    World {
        app: router(GameState::new(pool.clone(), verifier)),
        keys,
        pool,
    }
}

impl World {
    fn token(&self, handle: &str) -> String {
        self.keys.token_for(
            UserId::new(),
            handle,
            (Utc::now() + Duration::hours(1)).timestamp(),
        )
    }

    async fn get(&self, uri: &str) -> (StatusCode, Value) {
        self.call(Method::GET, uri, None).await
    }

    async fn call(&self, method: Method, uri: &str, token: Option<&str>) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            request = request.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        let response = self
            .app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn join(&self, id: &str, seat: &str, handle: &str) -> (StatusCode, Value) {
        let request = Request::builder()
            .method(Method::POST)
            .uri(format!("/instances/{id}/join"))
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", self.token(handle)),
            )
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json!({ "seat": seat }).to_string()))
            .unwrap();
        let response = self.app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }

    async fn open_lobby(&self, variant: &str, handle: &str) -> String {
        let (status, body) = self
            .call(
                Method::POST,
                &format!("/catalog/{variant}/lobbies"),
                Some(&self.token(handle)),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_str().unwrap().to_string()
    }
}

fn page(game: &str, preview: Option<&str>) -> GamePage {
    GamePage {
        slug: slug(game),
        name: Some(game.to_uppercase()),
        preview_variant: preview.map(slug),
        tagline: "t".to_string(),
        description: "d".to_string(),
        tags: vec![],
        accent: None,
        cover_image: None,
        skin: Some(json!({ "board": { "style": "game" } })),
        sections: vec![PageSection::Preview],
        featured: false,
        sort_order: 0,
    }
}

fn variant(game: &str, variant_slug: &str, label: &str, order: i32) -> Variant {
    Variant {
        slug: slug(variant_slug),
        game: slug(game),
        label: label.to_string(),
        sort_order: order,
        skin: None,
    }
}

async fn store_broken_rules(pool: &PgPool, variant_slug: &str) {
    sqlx::query(
        "INSERT INTO game_specs (id, slug, version, name, rules, rules_hash, active, created_at) \
         VALUES ($1, $2, 1, 'Broken', '{\"name\": \"nope\"}', 'x', TRUE, now())",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(variant_slug)
    .execute(pool)
    .await
    .unwrap();
}

fn slugs(list: &Value) -> Vec<&str> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["slug"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn the_front_page_lists_go_once_with_its_three_variants() {
    with_db(|pool| async move {
        let world = world(pool).await;

        let (status, list) = world.get("/catalog").await;

        assert_eq!(status, StatusCode::OK);
        let games = slugs(&list);
        assert!(games.contains(&"go"));
        assert!(
            !games.contains(&"go-13") && !games.contains(&"go-19"),
            "{games:?}"
        );
        let go = list
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["slug"] == "go")
            .unwrap();
        assert_eq!(go["name"], "Go");
        assert_eq!(
            go["variants"],
            json!([
                { "slug": "go", "label": "9x9" },
                { "slug": "go-13", "label": "13x13" },
                { "slug": "go-19", "label": "19x19" },
            ])
        );
    })
    .await;
}

#[tokio::test]
async fn the_game_page_describes_every_variant() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let thirteen = SpecRepo::new(world.pool.clone())
            .latest(&slug("go-13"))
            .await
            .unwrap()
            .unwrap();

        let (status, detail) = world.get("/catalog/go").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(detail["preview_variant"], "go");
        assert_eq!(detail["page"]["name"], "Go");
        assert_eq!(detail["spec"]["name"], "Go 9x9");
        let variants = detail["variants"].as_array().unwrap();
        assert_eq!(variants.len(), 3);
        assert_eq!(variants[0]["slug"], "go");
        assert!(variants[0]["skin"].is_null(), "9x9 uses the game's skin");
        assert!(variants[1]["skin"].is_object(), "13x13 has its own");
        assert_eq!(variants[1]["name"], json!(thirteen.name));
        assert_eq!(variants[1]["seats"], json!(["BLACK", "WHITE"]));
    })
    .await;
}

#[tokio::test]
async fn a_variant_slug_is_not_a_game_slug() {
    with_db(|pool| async move {
        let world = world(pool).await;

        let (status, body) = world.get("/catalog/go-13").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body, json!({ "error": "game not found" }));
    })
    .await;
}

#[tokio::test]
async fn a_lobby_can_be_opened_in_a_variant_that_has_no_page() {
    with_db(|pool| async move {
        let world = world(pool).await;

        let id = world.open_lobby("go-13", "Alice").await;

        let (status, instance) = world.get(&format!("/instances/{id}")).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(instance["slug"], "go-13");
        assert_eq!(instance["game"], "go");
        assert_eq!(instance["variant_label"], "13x13");
    })
    .await;
}

#[tokio::test]
async fn joining_answers_with_the_game_and_the_variant() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let id = world.open_lobby("go-19", "Alice").await;

        let (status, joined) = world.join(&id, "WHITE", "Bob").await;

        assert_eq!(status, StatusCode::OK, "{joined}");
        assert_eq!(joined["slug"], "go-19");
        assert_eq!(joined["game"], "go");
        assert_eq!(joined["variant_label"], "19x19");
        let (_, shown) = world.get(&format!("/instances/{id}")).await;
        assert_eq!(shown["status"], "active");
        assert_eq!(shown["game"], "go");
    })
    .await;
}

#[tokio::test]
async fn opening_a_lobby_in_an_unknown_variant_is_404() {
    with_db(|pool| async move {
        let world = world(pool).await;

        let (status, body) = world
            .call(
                Method::POST,
                "/catalog/nope/lobbies",
                Some(&world.token("Alice")),
            )
            .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body, json!({ "error": "game not found" }));
    })
    .await;
}

#[tokio::test]
async fn the_game_page_lists_lobbies_of_all_its_variants() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let nine = world.open_lobby("go", "Alice").await;
        let nineteen = world.open_lobby("go-19", "Bob").await;
        world.open_lobby("chess", "Carol").await;

        let (_, go) = world.get("/catalog/go").await;
        let (_, chess) = world.get("/catalog/chess").await;

        let lobbies = go["open_lobbies"].as_array().unwrap();
        let seen: Vec<_> = lobbies
            .iter()
            .map(|l| {
                (
                    l["id"].as_str().unwrap(),
                    l["variant"].as_str().unwrap(),
                    l["variant_label"].as_str().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            seen,
            [
                (nineteen.as_str(), "go-19", "19x19"),
                (nine.as_str(), "go", "9x9")
            ],
            "newest first, chess excluded"
        );
        assert_eq!(chess["open_lobbies"].as_array().unwrap().len(), 1);
        assert_eq!(chess["open_lobbies"][0]["variant"], "chess");
        assert_eq!(chess["open_lobbies"][0]["variant_label"], "Chess");
    })
    .await;
}

#[tokio::test]
async fn a_single_variant_game_has_one_variant_and_no_variant_skin() {
    with_db(|pool| async move {
        let world = world(pool).await;

        let (_, detail) = world.get("/catalog/chess").await;

        let variants = detail["variants"].as_array().unwrap();
        assert_eq!(variants.len(), 1);
        assert_eq!(variants[0]["slug"], "chess");
        assert_eq!(variants[0]["label"], "Chess");
        assert!(variants[0]["skin"].is_null());
        assert_eq!(detail["preview_variant"], "chess");
    })
    .await;
}

#[tokio::test]
async fn a_variant_that_cannot_be_read_is_left_out_and_the_game_stays() {
    with_db(|pool| async move {
        modern_chess_game::migrate(&pool).await.unwrap();
        SpecRepo::new(pool.clone())
            .upsert(
                slug("chess"),
                "Chess".to_string(),
                serde_json::from_str(CHESS).unwrap(),
                t0(),
            )
            .await
            .unwrap();
        store_broken_rules(&pool, "broken").await;
        PageRepo::new(pool.clone())
            .save_game(
                &page("mixed", Some("broken")),
                &[
                    variant("mixed", "broken", "Broken", 0),
                    variant("mixed", "chess", "Chess", 1),
                ],
            )
            .await
            .unwrap();
        let world = bare_world(pool);

        let (_, list) = world.get("/catalog").await;
        let (_, detail) = world.get("/catalog/mixed").await;

        assert_eq!(
            list[0]["variants"],
            json!([{ "slug": "chess", "label": "Chess" }])
        );
        assert_eq!(slugs(&detail["variants"]), ["chess"]);
        assert_eq!(
            detail["preview_variant"], "chess",
            "the preview falls back to a variant that can be played"
        );
        assert_eq!(detail["spec"]["name"], "Chess");
    })
    .await;
}

#[tokio::test]
async fn a_game_with_no_readable_variant_is_left_out() {
    with_db(|pool| async move {
        modern_chess_game::migrate(&pool).await.unwrap();
        store_broken_rules(&pool, "broken").await;
        PageRepo::new(pool.clone())
            .save_game(
                &page("broken", None),
                &[variant("broken", "broken", "Broken", 0)],
            )
            .await
            .unwrap();
        let world = bare_world(pool);

        let (_, list) = world.get("/catalog").await;
        let (status, _) = world.get("/catalog/broken").await;

        assert_eq!(list, json!([]));
        assert_eq!(status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn a_match_whose_variant_row_is_missing_still_answers() {
    with_db(|pool| async move {
        let world = world(pool).await;
        let id = world.open_lobby("go-13", "Alice").await;
        sqlx::query("DELETE FROM game_variants WHERE slug = 'go-13'")
            .execute(&world.pool)
            .await
            .unwrap();

        let (status, instance) = world.get(&format!("/instances/{id}")).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(instance["game"], "go-13");
        assert!(instance["variant_label"].is_null());
    })
    .await;
}
