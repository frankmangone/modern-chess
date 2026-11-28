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
            [
                "identities",
                "post_status_history",
                "posts",
                "threads",
                "users"
            ],
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

#[tokio::test]
async fn a_user_can_take_a_free_handle_and_the_old_one_is_free_again() {
    with_db(|pool| async move {
        modern_chess_social::migrate(&pool).await.unwrap();
        let users = UserRepo::new(pool);
        let alice = guest("Alice");
        users.insert(&alice).await.unwrap();

        users
            .set_handle(alice.id, &Handle::parse("Ada").unwrap())
            .await
            .unwrap();

        assert_eq!(
            users.get(alice.id).await.unwrap().unwrap().handle.as_str(),
            "Ada"
        );
        assert!(users
            .handle_is_free(&Handle::parse("alice").unwrap())
            .await
            .unwrap());
        assert!(
            !users
                .handle_is_free(&Handle::parse("ADA").unwrap())
                .await
                .unwrap(),
            "in any letter case"
        );
    })
    .await;
}

#[tokio::test]
async fn a_handle_someone_has_is_refused_in_any_letter_case_but_your_own_can_change_case() {
    with_db(|pool| async move {
        modern_chess_social::migrate(&pool).await.unwrap();
        let users = UserRepo::new(pool);
        let (alice, bob) = (guest("Alice"), guest("Bob"));
        users.insert(&alice).await.unwrap();
        users.insert(&bob).await.unwrap();

        let clash = users
            .set_handle(bob.id, &Handle::parse("ALICE").unwrap())
            .await;
        users
            .set_handle(alice.id, &Handle::parse("alice").unwrap())
            .await
            .unwrap();

        assert!(matches!(clash, Err(StoreError::HandleTaken)), "{clash:?}");
        assert_eq!(
            users.get(bob.id).await.unwrap().unwrap().handle.as_str(),
            "Bob",
            "unchanged"
        );
    })
    .await;
}

#[tokio::test]
async fn nobody_else_is_renamed_and_a_missing_user_is_reported() {
    with_db(|pool| async move {
        modern_chess_social::migrate(&pool).await.unwrap();
        let users = UserRepo::new(pool);

        let result = users
            .set_handle(UserId::new(), &Handle::parse("Ghost").unwrap())
            .await;

        assert!(matches!(result, Err(StoreError::UserMissing)), "{result:?}");
        assert!(!users.finish_setup(UserId::new()).await.unwrap());
    })
    .await;
}

#[tokio::test]
async fn finishing_the_set_up_clears_the_flag() {
    with_db(|pool| async move {
        modern_chess_social::migrate(&pool).await.unwrap();
        let users = UserRepo::new(pool);
        let account = User::new_account(Handle::parse("Player-1").unwrap(), t0());
        users.insert(&account).await.unwrap();
        assert!(users.get(account.id).await.unwrap().unwrap().setup_pending);

        assert!(users.finish_setup(account.id).await.unwrap());

        assert!(!users.get(account.id).await.unwrap().unwrap().setup_pending);
    })
    .await;
}
