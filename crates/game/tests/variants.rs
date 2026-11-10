//! Games that contain variants, against a real Postgres (`make db-up`).

use chrono::{DateTime, Duration, TimeZone, Utc};
use modern_chess_game::seed;
use modern_chess_game::store::{InstanceRepo, PageRepo, SpecRepo, VariantRepo};
use modern_chess_game::{GameInstance, GamePage, PageSection, Slug, Variant};
use modern_chess_testdb::with_db;
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

const CHESS: &str = include_str!("../../engine/specs/chess.json");
const CHECKERS: &str = include_str!("../../engine/specs/checkers.json");
const GO: &str = include_str!("../../engine/specs/go.json");
const GO_13: &str = include_str!("../../engine/specs/go_13.json");
const GO_19: &str = include_str!("../../engine/specs/go_19.json");
const GO_SPECS: [(&str, &str); 3] = [("go", GO), ("go_13", GO_13), ("go_19", GO_19)];

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).unwrap()
}

fn slug(raw: &str) -> Slug {
    Slug::parse(raw).unwrap()
}

fn page(game: &str) -> GamePage {
    GamePage {
        slug: slug(game),
        name: Some(game.to_uppercase()),
        preview_variant: Some(slug(game)),
        tagline: format!("About {game}"),
        description: "Long text.".to_string(),
        tags: vec![],
        accent: None,
        cover_image: None,
        skin: None,
        sections: vec![PageSection::Preview],
        featured: false,
        sort_order: 0,
    }
}

fn variant(game: &str, slug_: &str, label: &str, sort_order: i32) -> Variant {
    Variant {
        slug: slug(slug_),
        game: slug(game),
        label: label.to_string(),
        sort_order,
        skin: None,
    }
}

async fn migrated(pool: &PgPool) {
    modern_chess_game::migrate(pool).await.unwrap();
}

async fn store_spec(pool: &PgPool, slug_: &str, name: &str, rules: &str) {
    SpecRepo::new(pool.clone())
        .upsert(
            slug(slug_),
            name.to_string(),
            serde_json::from_str(rules).unwrap(),
            t0(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn migration_backfills_one_variant_per_existing_page() {
    with_db(|pool| async move {
        sqlx::raw_sql(include_str!("../migrations/0001_init.up.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO game_specs (id, slug, version, name, rules, rules_hash, active, created_at) \
             VALUES ($1, 'chess', 1, 'Chess', '{}', 'x', TRUE, now())",
        )
        .bind(uuid::Uuid::new_v4())
        .execute(&pool)
        .await
        .unwrap();
        for game in ["chess", "orphan"] {
            sqlx::query(
                "INSERT INTO game_pages (slug, tagline, description, tags, sections, featured, sort_order) \
                 VALUES ($1, 't', 'd', '{}', '[]', FALSE, 0)",
            )
            .bind(game)
            .execute(&pool)
            .await
            .unwrap();
        }

        sqlx::raw_sql(include_str!("../migrations/0002_variants.up.sql"))
            .execute(&pool)
            .await
            .unwrap();

        let variants = sqlx::query(
            "SELECT slug, game_slug, label, sort_order FROM game_variants ORDER BY slug",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let rows: Vec<(String, String, String, i32)> = variants
            .iter()
            .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3)))
            .collect();
        assert_eq!(
            rows,
            [
                ("chess".into(), "chess".into(), "Chess".into(), 0),
                ("orphan".into(), "orphan".into(), "orphan".into(), 0),
            ]
        );
        let names: Vec<String> = sqlx::query("SELECT name FROM game_pages ORDER BY slug")
            .fetch_all(&pool)
            .await
            .unwrap()
            .iter()
            .map(|r| r.get(0))
            .collect();
        assert_eq!(names, ["Chess", "orphan"]);
    })
    .await;
}

#[tokio::test]
async fn the_down_migration_removes_what_the_up_migration_added() {
    with_db(|pool| async move {
        sqlx::raw_sql(include_str!("../migrations/0001_init.up.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::raw_sql(include_str!("../migrations/0002_variants.up.sql"))
            .execute(&pool)
            .await
            .unwrap();

        sqlx::raw_sql(include_str!("../migrations/0002_variants.down.sql"))
            .execute(&pool)
            .await
            .unwrap();

        let table: Option<String> = sqlx::query_scalar("SELECT to_regclass('game_variants')::text")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(table, None);
        let columns: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM information_schema.columns \
             WHERE table_name = 'game_pages' AND column_name IN ('name', 'preview_variant')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(columns, 0);
    })
    .await;
}

#[tokio::test]
async fn save_game_stores_the_page_and_its_variants_together() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let pages = PageRepo::new(pool.clone());
        let variants = VariantRepo::new(pool.clone());
        let mut thirteen = variant("go", "go-13", "13x13", 1);
        thirteen.skin = Some(json!({ "board": { "x": 1 } }));
        let all = [variant("go", "go", "9x9", 0), thirteen];

        pages.save_game(&page("go"), &all).await.unwrap();

        assert_eq!(variants.for_game(&slug("go")).await.unwrap(), all);
        assert_eq!(variants.all().await.unwrap(), all);
        assert_eq!(
            variants.get(&slug("go-13")).await.unwrap().unwrap().label,
            "13x13"
        );
        assert_eq!(variants.get(&slug("nope")).await.unwrap(), None);
        assert_eq!(pages.get(&slug("go")).await.unwrap(), Some(page("go")));
    })
    .await;
}

#[tokio::test]
async fn saving_a_game_moves_a_variant_that_belonged_to_another_game() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let pages = PageRepo::new(pool.clone());
        let variants = VariantRepo::new(pool.clone());
        pages
            .save_game(&page("go-13"), &[Variant::single(&slug("go-13"), "Go")])
            .await
            .unwrap();

        pages
            .save_game(
                &page("go"),
                &[
                    variant("go", "go", "9x9", 0),
                    variant("go", "go-13", "13x13", 1),
                ],
            )
            .await
            .unwrap();

        assert_eq!(
            variants.get(&slug("go-13")).await.unwrap().unwrap().game,
            slug("go")
        );
        assert_eq!(pages.get(&slug("go-13")).await.unwrap(), None);
        assert!(pages.get(&slug("go")).await.unwrap().is_some());
    })
    .await;
}

#[tokio::test]
async fn a_variant_moved_out_of_a_game_that_keeps_others_does_not_delete_that_game() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let pages = PageRepo::new(pool.clone());
        let variants = VariantRepo::new(pool.clone());
        pages
            .save_game(
                &page("a"),
                &[variant("a", "a", "A", 0), variant("a", "b", "B", 1)],
            )
            .await
            .unwrap();

        pages
            .save_game(&page("c"), &[variant("c", "b", "B", 0)])
            .await
            .unwrap();

        assert!(pages.get(&slug("a")).await.unwrap().is_some());
        let left: Vec<_> = variants
            .for_game(&slug("a"))
            .await
            .unwrap()
            .into_iter()
            .map(|v| v.slug)
            .collect();
        assert_eq!(left, [slug("a")]);
    })
    .await;
}

#[tokio::test]
async fn saving_a_game_twice_changes_nothing_and_keeps_its_cover() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let pages = PageRepo::new(pool.clone());
        let variants = VariantRepo::new(pool.clone());
        let all = [
            variant("go", "go", "9x9", 0),
            variant("go", "go-13", "13x13", 1),
        ];
        pages.save_game(&page("go"), &all).await.unwrap();
        pages
            .set_cover_image(&slug("go"), Some("covers/go/a.jpg"))
            .await
            .unwrap();

        pages.save_game(&page("go"), &all).await.unwrap();

        let stored = pages.get(&slug("go")).await.unwrap().unwrap();
        assert_eq!(stored.cover_image.as_deref(), Some("covers/go/a.jpg"));
        assert_eq!(
            GamePage {
                cover_image: None,
                ..stored
            },
            page("go")
        );
        assert_eq!(variants.all().await.unwrap(), all);
    })
    .await;
}

#[tokio::test]
async fn open_lobbies_spans_several_variants_newest_first() {
    with_db(|pool| async move {
        migrated(&pool).await;
        store_spec(&pool, "chess", "Chess", CHESS).await;
        store_spec(&pool, "checkers", "Checkers", CHECKERS).await;
        let specs = SpecRepo::new(pool.clone());
        let instances = InstanceRepo::new(pool.clone());
        let now = Utc::now();
        let older = GameInstance::new_lobby(
            &specs.latest(&slug("chess")).await.unwrap().unwrap(),
            now - Duration::minutes(5),
        )
        .unwrap();
        let newer = GameInstance::new_lobby(
            &specs.latest(&slug("checkers")).await.unwrap().unwrap(),
            now - Duration::minutes(1),
        )
        .unwrap();
        instances.create(&older).await.unwrap();
        instances.create(&newer).await.unwrap();
        let cutoff = now - Duration::hours(1);

        let both = instances
            .open_lobbies(&[slug("chess"), slug("checkers")], cutoff, 50)
            .await
            .unwrap();
        let only_chess = instances
            .open_lobbies(&[slug("chess")], cutoff, 50)
            .await
            .unwrap();

        let ids: Vec<Value> = both.iter().map(|l| json!(l.id().to_string())).collect();
        assert_eq!(
            ids,
            [json!(newer.id().to_string()), json!(older.id().to_string())]
        );
        assert_eq!(only_chess.len(), 1);
        assert_eq!(only_chess[0].id(), older.id());
    })
    .await;
}

fn old_go_page(game: &str) -> String {
    json!({ "slug": game, "tagline": "Old", "description": "d" }).to_string()
}

fn new_go_page() -> String {
    json!({
        "slug": "go",
        "name": "Go",
        "preview_variant": "go",
        "tagline": "New",
        "description": "d",
        "variants": [
            { "slug": "go", "label": "9x9" },
            { "slug": "go-13", "label": "13x13", "skin": { "board": { "style": "intersections" } } },
            { "slug": "go-19", "label": "19x19" }
        ]
    })
    .to_string()
}

async fn seed_pages(
    pool: &PgPool,
    spec_files: &[(&str, &str)],
    page_files: &[&str],
) -> seed::SeedReport {
    let named: Vec<(&str, &str)> = page_files.iter().map(|text| ("page", *text)).collect();
    seed::run_with(
        &SpecRepo::new(pool.clone()),
        &PageRepo::new(pool.clone()),
        spec_files,
        &named,
        t0(),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn a_page_file_without_variants_is_one_variant_named_after_its_rules() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let page = json!({ "slug": "chess", "tagline": "t", "description": "d" }).to_string();

        let report = seed_pages(&pool, &[("chess", CHESS)], &[&page]).await;

        assert_eq!((report.pages, report.variants), (1, 1));
        assert_eq!(
            VariantRepo::new(pool.clone())
                .for_game(&slug("chess"))
                .await
                .unwrap(),
            [Variant::single(&slug("chess"), "Chess")]
        );
        let stored = PageRepo::new(pool.clone())
            .get(&slug("chess"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.name.as_deref(), Some("Chess"));
    })
    .await;
}

#[tokio::test]
async fn a_page_file_with_variants_saves_them_in_order_with_skins() {
    with_db(|pool| async move {
        migrated(&pool).await;

        let report = seed_pages(&pool, &GO_SPECS, &[&new_go_page()]).await;

        assert_eq!((report.pages, report.variants), (1, 3));
        assert!(report.skipped.is_empty(), "{:?}", report.skipped);
        let variants = VariantRepo::new(pool.clone())
            .for_game(&slug("go"))
            .await
            .unwrap();
        let summary: Vec<_> = variants
            .iter()
            .map(|v| {
                (
                    v.slug.as_str(),
                    v.label.as_str(),
                    v.sort_order,
                    v.skin.is_some(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("go", "9x9", 0, false),
                ("go-13", "13x13", 1, true),
                ("go-19", "19x19", 2, false)
            ]
        );
        let stored = PageRepo::new(pool.clone())
            .get(&slug("go"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.name.as_deref(), Some("Go"));
        assert_eq!(stored.preview_variant, Some(slug("go")));
    })
    .await;
}

#[tokio::test]
async fn a_game_without_a_name_takes_the_name_of_its_first_variants_rules() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let page = json!({
            "slug": "go", "tagline": "t", "description": "d",
            "variants": [{ "slug": "go-13", "label": "13x13" }]
        })
        .to_string();

        seed_pages(&pool, &GO_SPECS, &[&page]).await;

        let stored = PageRepo::new(pool.clone())
            .get(&slug("go"))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stored.name.as_deref(), Some("Go 13x13"));
    })
    .await;
}

#[tokio::test]
async fn a_preview_variant_that_is_not_listed_skips_that_page_only() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let bad = json!({
            "slug": "go", "tagline": "t", "description": "d", "preview_variant": "nope",
            "variants": [{ "slug": "go", "label": "9x9" }]
        })
        .to_string();
        let good = json!({ "slug": "chess", "tagline": "t", "description": "d" }).to_string();

        let report = seed_pages(&pool, &[("chess", CHESS), ("go", GO)], &[&bad, &good]).await;

        assert_eq!(report.pages, 1);
        assert_eq!(report.skipped.len(), 1, "{:?}", report.skipped);
        assert_eq!(
            PageRepo::new(pool.clone()).get(&slug("go")).await.unwrap(),
            None
        );
        assert!(PageRepo::new(pool.clone())
            .get(&slug("chess"))
            .await
            .unwrap()
            .is_some());
    })
    .await;
}

#[tokio::test]
async fn duplicate_variant_slugs_in_one_file_skip_the_page() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let page = json!({
            "slug": "go", "tagline": "t", "description": "d",
            "variants": [{ "slug": "go", "label": "9x9" }, { "slug": "go", "label": "again" }]
        })
        .to_string();

        let report = seed_pages(&pool, &GO_SPECS, &[&page]).await;

        assert_eq!(report.pages, 0);
        assert_eq!(report.skipped.len(), 1, "{:?}", report.skipped);
    })
    .await;
}

#[tokio::test]
async fn the_old_go_pages_become_variants_of_go_and_vanish_as_pages() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let pages = PageRepo::new(pool.clone());
        let old = [
            old_go_page("go"),
            old_go_page("go-13"),
            old_go_page("go-19"),
        ];
        seed_pages(&pool, &GO_SPECS, &[&old[0], &old[1], &old[2]]).await;
        pages
            .set_cover_image(&slug("go"), Some("covers/go/a.jpg"))
            .await
            .unwrap();
        let specs = SpecRepo::new(pool.clone());
        let lobby =
            GameInstance::new_lobby(&specs.latest(&slug("go-13")).await.unwrap().unwrap(), t0())
                .unwrap();
        InstanceRepo::new(pool.clone())
            .create(&lobby)
            .await
            .unwrap();

        let report = seed_pages(&pool, &GO_SPECS, &[&new_go_page()]).await;

        assert!(report.skipped.is_empty(), "{:?}", report.skipped);
        let listed: Vec<_> = pages
            .list()
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.slug)
            .collect();
        assert_eq!(listed, [slug("go")]);
        let variants: Vec<_> = VariantRepo::new(pool.clone())
            .for_game(&slug("go"))
            .await
            .unwrap()
            .into_iter()
            .map(|v| v.slug)
            .collect();
        assert_eq!(variants, [slug("go"), slug("go-13"), slug("go-19")]);
        let stored = pages.get(&slug("go")).await.unwrap().unwrap();
        assert_eq!(stored.cover_image.as_deref(), Some("covers/go/a.jpg"));
        assert!(InstanceRepo::new(pool.clone())
            .get(lobby.id())
            .await
            .unwrap()
            .is_some());
    })
    .await;
}

#[tokio::test]
async fn seeding_the_regrouped_game_twice_changes_nothing() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let pages = PageRepo::new(pool.clone());
        let variants = VariantRepo::new(pool.clone());
        seed_pages(&pool, &GO_SPECS, &[&new_go_page()]).await;
        let (pages_before, variants_before) =
            (pages.list().await.unwrap(), variants.all().await.unwrap());

        seed_pages(&pool, &GO_SPECS, &[&new_go_page()]).await;

        assert_eq!(pages.list().await.unwrap(), pages_before);
        assert_eq!(variants.all().await.unwrap(), variants_before);
    })
    .await;
}
