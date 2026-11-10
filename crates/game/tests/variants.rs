//! Games that contain variants, against a real Postgres (`make db-up`).

use chrono::{DateTime, Duration, TimeZone, Utc};
use modern_chess_game::store::{InstanceRepo, PageRepo, SpecRepo, VariantRepo};
use modern_chess_game::{GameInstance, GamePage, PageSection, Slug, Variant};
use modern_chess_testdb::with_db;
use serde_json::{json, Value};
use sqlx::{PgPool, Row};

const CHESS: &str = include_str!("../../engine/specs/chess.json");
const CHECKERS: &str = include_str!("../../engine/specs/checkers.json");

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
