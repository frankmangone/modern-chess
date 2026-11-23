//! Stored rules gain the `format_version` key, against a real Postgres (`make db-up`).

use modern_chess_testdb::with_db;
use serde_json::{json, Value};
use sqlx::PgPool;

async fn migrate_up_to_the_key(pool: &PgPool) {
    for sql in [
        include_str!("../migrations/0001_init.up.sql"),
        include_str!("../migrations/0002_variants.up.sql"),
        include_str!("../migrations/0003_families.up.sql"),
        include_str!("../migrations/0004_position_names.up.sql"),
        include_str!("../migrations/0005_actions.up.sql"),
    ] {
        sqlx::raw_sql(sql).execute(pool).await.unwrap();
    }
}

async fn store(pool: &PgPool, slug: &str, rules: Value) {
    sqlx::query(
        "INSERT INTO game_specs (id, slug, version, name, rules, rules_hash, active, created_at) \
         VALUES (gen_random_uuid(), $1, 1, $1, $2, 'old-hash', TRUE, now())",
    )
    .bind(slug)
    .bind(rules)
    .execute(pool)
    .await
    .unwrap();
}

async fn rules_of(pool: &PgPool, slug: &str) -> Value {
    sqlx::query_scalar("SELECT rules FROM game_specs WHERE slug = $1")
        .bind(slug)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn stored_rules_without_a_version_are_given_version_one() {
    with_db(|pool| async move {
        migrate_up_to_the_key(&pool).await;
        store(
            &pool,
            "old",
            json!({ "name": "OLD", "board": { "dimensions": [2, 2] } }),
        )
        .await;
        store(&pool, "new", json!({ "format_version": 1, "name": "NEW" })).await;

        sqlx::raw_sql(include_str!("../migrations/0006_format_version.up.sql"))
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(
            rules_of(&pool, "old").await,
            json!({ "format_version": 1, "name": "OLD", "board": { "dimensions": [2, 2] } })
        );
        assert_eq!(
            rules_of(&pool, "new").await,
            json!({ "format_version": 1, "name": "NEW" })
        );
        let hash: String =
            sqlx::query_scalar("SELECT rules_hash FROM game_specs WHERE slug = 'old'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(hash, "old-hash", "the hash is left as it was");
    })
    .await;
}

#[tokio::test]
async fn the_down_migration_takes_the_key_out_again() {
    with_db(|pool| async move {
        migrate_up_to_the_key(&pool).await;
        store(&pool, "old", json!({ "name": "OLD" })).await;
        sqlx::raw_sql(include_str!("../migrations/0006_format_version.up.sql"))
            .execute(&pool)
            .await
            .unwrap();

        sqlx::raw_sql(include_str!("../migrations/0006_format_version.down.sql"))
            .execute(&pool)
            .await
            .unwrap();

        assert_eq!(rules_of(&pool, "old").await, json!({ "name": "OLD" }));
    })
    .await;
}
