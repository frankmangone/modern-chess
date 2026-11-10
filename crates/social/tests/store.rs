//! The user repository against a real Postgres (`make db-up`), one throwaway database per test.

use chrono::{DateTime, TimeZone, Utc};
use modern_chess_kernel::{Handle, UserId};
use modern_chess_social::store::{StoreError, UserRepo};
use modern_chess_social::User;
use modern_chess_testdb::with_db;

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap()
}

fn guest(name: &str) -> User {
    User::new_guest(Handle::parse(name).unwrap(), t0())
}

#[tokio::test]
async fn social_migrates_its_own_database() {
    with_db(|pool| async move {
        modern_chess_social::migrate(&pool).await.unwrap();
        modern_chess_social::migrate(&pool)
            .await
            .expect("migrating twice is harmless");

        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT table_name::text FROM information_schema.tables \
             WHERE table_schema = 'public' AND table_name NOT LIKE '\\_sqlx%' ORDER BY 1",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(
            tables,
            ["identities", "users"],
            "only social tables, none of game's"
        );
    })
    .await;
}

#[tokio::test]
async fn guest_user_roundtrips() {
    with_db(|pool| async move {
        modern_chess_social::migrate(&pool).await.unwrap();
        let users = UserRepo::new(pool);
        let alice = guest("Alice");

        users.insert(&alice).await.unwrap();

        assert_eq!(users.get(alice.id).await.unwrap(), Some(alice));
        assert_eq!(users.get(UserId::new()).await.unwrap(), None);
    })
    .await;
}

#[tokio::test]
async fn handles_are_unique_and_a_clash_is_reported_not_swallowed() {
    with_db(|pool| async move {
        modern_chess_social::migrate(&pool).await.unwrap();
        let users = UserRepo::new(pool);
        let first = guest("Guest-1001");
        users.insert(&first).await.unwrap();

        let same = users.insert(&guest("Guest-1001")).await;
        let other_case = users.insert(&guest("guest-1001")).await;

        assert!(matches!(same, Err(StoreError::HandleTaken)));
        assert!(
            matches!(other_case, Err(StoreError::HandleTaken)),
            "handles clash regardless of case"
        );
        assert_eq!(users.get(first.id).await.unwrap(), Some(first));
    })
    .await;
}
