//! The read-only catalogue over HTTP, against a real Postgres (`make db-up`).

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::Router;
use chrono::{DateTime, TimeZone, Utc};
use modern_chess_game::seed;
use modern_chess_game::store::{PageRepo, SpecRepo};
use modern_chess_game::{router, GamePage, GameState, PageSection, Slug};
use modern_chess_kernel::TokenVerifier;
use modern_chess_testdb::with_db;
use modern_chess_testkeys::TestKeys;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

const CHESS: &str = include_str!("../../engine/specs/chess.json");
const CHECKERS: &str = include_str!("../../engine/specs/checkers.json");
const ATAXX: &str = include_str!("../../engine/specs/ataxx.json");

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap()
}

async fn ready(pool: &PgPool) -> Router {
    modern_chess_game::migrate(pool).await.unwrap();
    let verifier = TokenVerifier::from_public_pem(&TestKeys::generate().public_pem).unwrap();
    router(GameState::new(pool.clone(), verifier))
}

async fn ready_with_images(pool: &PgPool, base: &str) -> Router {
    modern_chess_game::migrate(pool).await.unwrap();
    let verifier = TokenVerifier::from_public_pem(&TestKeys::generate().public_pem).unwrap();
    router(GameState::new(pool.clone(), verifier).with_image_base(base))
}

async fn store_spec(pool: &PgPool, slug: &str, name: &str, rules: &str) {
    SpecRepo::new(pool.clone())
        .upsert(
            Slug::parse(slug).unwrap(),
            name.to_string(),
            serde_json::from_str(rules).unwrap(),
            t0(),
        )
        .await
        .unwrap();
}

fn page(slug: &str, sort_order: i32) -> GamePage {
    GamePage {
        name: None,
        preview_variant: None,
        slug: Slug::parse(slug).unwrap(),
        tagline: format!("About {slug}"),
        description: "Long text.".to_string(),
        tags: vec!["classic".to_string()],
        accent: Some("#112233".to_string()),
        cover_image: None,
        skin: None,
        sections: vec![
            PageSection::HowToPlay {
                markdown: "Move.".to_string(),
            },
            PageSection::Preview,
            PageSection::OpenLobbies,
        ],
        featured: false,
        sort_order,
    }
}

async fn store_page(pool: &PgPool, page: &GamePage) {
    PageRepo::new(pool.clone()).upsert(page).await.unwrap();
}

async fn get(app: &Router, uri: &str) -> (StatusCode, Value) {
    let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn slugs(list: &Value) -> Vec<&str> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["slug"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn catalog_lists_only_games_that_have_a_page() {
    with_db(|pool| async move {
        let app = ready(&pool).await;
        store_spec(&pool, "chess", "Chess", CHESS).await;
        store_spec(&pool, "checkers", "Checkers", CHECKERS).await;
        store_page(&pool, &page("chess", 0)).await;

        let (status, list) = get(&app, "/catalog").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(slugs(&list), ["chess"]);
        let entry = &list[0];
        assert_eq!(entry["name"], "Chess");
        assert_eq!(entry["tagline"], "About chess");
        assert_eq!(entry["tags"], json!(["classic"]));
        assert_eq!(entry["accent"], "#112233");
        assert_eq!(entry["featured"], false);
        assert!(entry["cover_image"].is_null());
    })
    .await;
}

#[tokio::test]
async fn catalog_orders_by_sort_order_then_name() {
    with_db(|pool| async move {
        let app = ready(&pool).await;
        store_spec(&pool, "chess", "Chess", CHESS).await;
        store_spec(&pool, "checkers", "Checkers", CHECKERS).await;
        store_spec(&pool, "ataxx", "Ataxx", ATAXX).await;
        store_page(&pool, &page("chess", 1)).await;
        store_page(&pool, &page("checkers", 1)).await;
        store_page(&pool, &page("ataxx", 5)).await;

        let (_, list) = get(&app, "/catalog").await;

        assert_eq!(slugs(&list), ["checkers", "chess", "ataxx"]);
    })
    .await;
}

#[tokio::test]
async fn catalog_detail_returns_page_sections_in_order() {
    with_db(|pool| async move {
        let app = ready(&pool).await;
        store_spec(&pool, "chess", "Chess", CHESS).await;
        store_page(&pool, &page("chess", 0)).await;

        let (status, detail) = get(&app, "/catalog/chess").await;

        assert_eq!(status, StatusCode::OK);
        let types: Vec<_> = detail["page"]["sections"]
            .as_array()
            .unwrap()
            .iter()
            .map(|section| section["type"].as_str().unwrap())
            .collect();
        assert_eq!(types, ["how_to_play", "preview", "open_lobbies"]);
        assert_eq!(detail["page"]["tagline"], "About chess");
        assert_eq!(detail["spec"]["name"], "Chess");
        assert_eq!(detail["spec"]["version"], 1);
        assert_eq!(detail["spec"]["seats"], json!(["WHITE", "BLACK"]));
        assert_eq!(detail["spec"]["rules"]["name"], "CHESS");
        assert_eq!(detail["open_lobbies"], json!([]));
    })
    .await;
}

#[tokio::test]
async fn unknown_slug_is_404_with_a_plain_body() {
    with_db(|pool| async move {
        let app = ready(&pool).await;

        for uri in ["/catalog/nope", "/catalog/Not%20A%20Slug"] {
            let (status, body) = get(&app, uri).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{uri}");
            assert_eq!(body, json!({ "error": "game not found" }), "{uri}");
        }
    })
    .await;
}

#[tokio::test]
async fn a_spec_that_no_longer_parses_is_skipped_and_the_rest_still_load() {
    with_db(|pool| async move {
        let app = ready(&pool).await;
        store_spec(&pool, "chess", "Chess", CHESS).await;
        store_page(&pool, &page("chess", 0)).await;
        // A row the engine cannot read, as if the rules format had moved on since it was stored.
        sqlx::query(
            "INSERT INTO game_specs (id, slug, version, name, rules, rules_hash, active, created_at) \
             VALUES ($1, 'broken', 1, 'Broken', '{\"name\": \"nope\"}', 'x', TRUE, now())",
        )
        .bind(uuid::Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap();
        store_page(&pool, &page("broken", 1)).await;

        let (status, list) = get(&app, "/catalog").await;
        let (broken_status, _) = get(&app, "/catalog/broken").await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(slugs(&list), ["chess"]);
        assert_eq!(broken_status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn reseeding_changed_page_text_updates_it_without_a_new_spec_version() {
    with_db(|pool| async move {
        let app = ready(&pool).await;
        let specs = SpecRepo::new(pool.clone());
        let pages = PageRepo::new(pool.clone());
        let spec_files = [("chess", CHESS)];
        let before =
            json!({ "slug": "chess", "tagline": "Before", "description": "d" }).to_string();
        let after = json!({ "slug": "chess", "tagline": "After", "description": "d" }).to_string();

        seed::run_with(
            &specs,
            &pages,
            &spec_files,
            &[("chess", before.as_str())],
            t0(),
        )
        .await
        .unwrap();
        seed::run_with(
            &specs,
            &pages,
            &spec_files,
            &[("chess", after.as_str())],
            t0(),
        )
        .await
        .unwrap();

        let (_, detail) = get(&app, "/catalog/chess").await;
        assert_eq!(detail["page"]["tagline"], "After");
        assert_eq!(detail["spec"]["version"], 1);
    })
    .await;
}

#[tokio::test]
async fn a_broken_seed_file_is_reported_and_does_not_stop_the_others() {
    with_db(|pool| async move {
        let _app = ready(&pool).await;
        let specs = SpecRepo::new(pool.clone());
        let pages = PageRepo::new(pool.clone());
        let good = json!({ "slug": "chess", "tagline": "t", "description": "d" }).to_string();

        let report = seed::run_with(
            &specs,
            &pages,
            &[("broken", "{ not json"), ("chess", CHESS)],
            &[("chess", good.as_str()), ("oops", "[]")],
            t0(),
        )
        .await
        .unwrap();

        assert_eq!(report.specs, 1);
        assert_eq!(report.pages, 1);
        assert_eq!(report.skipped.len(), 2, "{:?}", report.skipped);
    })
    .await;
}

#[tokio::test]
async fn embedded_seed_files_publish_the_launch_games() {
    with_db(|pool| async move {
        let app = ready(&pool).await;
        let report = seed::run(
            &SpecRepo::new(pool.clone()),
            &PageRepo::new(pool.clone()),
            t0(),
        )
        .await
        .unwrap();
        assert!(report.skipped.is_empty(), "{:?}", report.skipped);

        let (_, list) = get(&app, "/catalog").await;
        let games = [
            "chess",
            "checkers",
            "ataxx",
            "chaturanga",
            "shogi",
            "atomic",
            "xiangqi",
            "go",
            "go-13",
            "go-19",
        ];
        assert_eq!(slugs(&list), games);

        for slug in games {
            let (status, detail) = get(&app, &format!("/catalog/{slug}")).await;
            assert_eq!(status, StatusCode::OK, "{slug}");
            let types: Vec<_> = detail["page"]["sections"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["type"].as_str().unwrap())
                .collect();
            assert_eq!(types, ["how_to_play", "preview", "open_lobbies"], "{slug}");
            assert!(detail["page"]["skin"].is_object(), "{slug} has a skin");
            assert!(detail["page"]["accent"].is_string(), "{slug} has an accent");
        }
    })
    .await;
}

async fn game_with_cover(pool: &PgPool, cover: Option<&str>) {
    store_spec(pool, "chess", "Chess", CHESS).await;
    let mut chess = page("chess", 0);
    chess.cover_image = cover.map(str::to_string);
    store_page(pool, &chess).await;
}

#[tokio::test]
async fn a_stored_cover_key_is_served_as_a_full_url_in_the_list_and_on_the_page() {
    with_db(|pool| async move {
        let app = ready_with_images(&pool, "http://localhost:9000/modern-chess").await;
        game_with_cover(&pool, Some("covers/chess/abc123.jpg")).await;
        let url = "http://localhost:9000/modern-chess/covers/chess/abc123.jpg";

        let (_, list) = get(&app, "/catalog").await;
        let (_, detail) = get(&app, "/catalog/chess").await;

        assert_eq!(list[0]["cover_image"], url);
        assert_eq!(detail["page"]["cover_image"], url);
    })
    .await;
}

#[tokio::test]
async fn slashes_at_the_seam_between_the_base_and_the_key_make_exactly_one() {
    with_db(|pool| async move {
        let app = ready_with_images(&pool, "http://img.test/bucket/").await;
        game_with_cover(&pool, Some("/covers/chess/abc.jpg")).await;

        let (_, list) = get(&app, "/catalog").await;

        assert_eq!(
            list[0]["cover_image"],
            "http://img.test/bucket/covers/chess/abc.jpg"
        );
    })
    .await;
}

#[tokio::test]
async fn a_stored_value_is_always_a_key_under_the_base_never_an_address_of_its_own() {
    with_db(|pool| async move {
        let app = ready_with_images(&pool, "http://img.test/bucket").await;
        game_with_cover(&pool, Some("https://elsewhere.test/pixel.png")).await;

        let (_, list) = get(&app, "/catalog").await;

        let url = list[0]["cover_image"].as_str().unwrap();
        assert!(url.starts_with("http://img.test/bucket/"), "{url}");
    })
    .await;
}

#[tokio::test]
async fn a_game_without_a_cover_stays_without_one() {
    with_db(|pool| async move {
        let app = ready_with_images(&pool, "http://img.test/bucket").await;
        game_with_cover(&pool, None).await;

        let (_, list) = get(&app, "/catalog").await;
        let (_, detail) = get(&app, "/catalog/chess").await;

        assert!(list[0]["cover_image"].is_null());
        assert!(detail["page"]["cover_image"].is_null());
    })
    .await;
}

#[tokio::test]
async fn without_an_image_base_the_stored_value_is_passed_on_as_it_is() {
    with_db(|pool| async move {
        let app = ready(&pool).await;
        game_with_cover(&pool, Some("covers/chess/abc.jpg")).await;

        let (_, list) = get(&app, "/catalog").await;

        assert_eq!(list[0]["cover_image"], "covers/chess/abc.jpg");
    })
    .await;
}

#[tokio::test]
async fn restarting_the_app_does_not_wipe_an_uploaded_cover() {
    with_db(|pool| async move {
        let app = ready(&pool).await;
        let specs = SpecRepo::new(pool.clone());
        let pages = PageRepo::new(pool.clone());
        let spec_files = [("chess", CHESS)];
        let page_files = [(
            "chess",
            json!({ "slug": "chess", "tagline": "t", "description": "d" }).to_string(),
        )];
        let page_files: Vec<(&str, &str)> =
            page_files.iter().map(|(n, t)| (*n, t.as_str())).collect();
        seed::run_with(&specs, &pages, &spec_files, &page_files, t0())
            .await
            .unwrap();
        let found = pages
            .set_cover_image(&Slug::parse("chess").unwrap(), Some("covers/chess/a.jpg"))
            .await
            .unwrap();
        assert!(found, "the first seed must have saved the page");

        // The next start seeds again.
        seed::run_with(&specs, &pages, &spec_files, &page_files, t0())
            .await
            .unwrap();

        let (_, detail) = get(&app, "/catalog/chess").await;
        assert_eq!(detail["page"]["cover_image"], "covers/chess/a.jpg");
    })
    .await;
}
