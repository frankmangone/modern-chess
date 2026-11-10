//! Signing in with a provider identity: who the user is, and what happens to a guest.
//! Against a real Postgres (`make db-up`).

use chrono::{DateTime, TimeZone, Utc};
use modern_chess_kernel::UserId;
use modern_chess_social::accounts::{delete_account, sign_in};
use modern_chess_social::guests::create_guest;
use modern_chess_social::provider::ExternalIdentity;
use modern_chess_social::store::{AccountRepo, StoreError, UserRepo};
use modern_chess_social::User;
use modern_chess_testdb::with_db;
use sqlx::PgPool;

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).unwrap()
}

fn identity(subject: &str) -> ExternalIdentity {
    ExternalIdentity {
        provider: "testidp".to_string(),
        subject: subject.to_string(),
        email: Some(format!("{subject}@example.test")),
    }
}

struct Repos {
    users: UserRepo,
    accounts: AccountRepo,
}

async fn repos(pool: &PgPool) -> Repos {
    modern_chess_social::migrate(pool).await.unwrap();
    Repos {
        users: UserRepo::new(pool.clone()),
        accounts: AccountRepo::new(pool.clone()),
    }
}

impl Repos {
    async fn sign_in(&self, who: &ExternalIdentity, guest: Option<UserId>) -> User {
        sign_in(&self.users, &self.accounts, who, guest, t0(), counter())
            .await
            .unwrap()
    }

    async fn guest(&self) -> User {
        create_guest(&self.users, t0(), counter()).await.unwrap()
    }
}

/// Handle digits that never repeat, so a clash can only come from the code under test.
fn counter() -> impl FnMut(u32) -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(1000);
    |_| NEXT.fetch_add(1, Ordering::SeqCst)
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT count(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap()
}

#[tokio::test]
async fn a_first_sign_in_makes_a_user_with_a_generated_handle() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;

        let user = repos.sign_in(&identity("user_1"), None).await;

        assert!(
            user.handle.as_str().starts_with("Player-"),
            "{}",
            user.handle
        );
        assert_eq!(repos.users.get(user.id).await.unwrap(), Some(user));
        assert_eq!(count(&pool, "identities").await, 1);
    })
    .await;
}

#[tokio::test]
async fn signing_in_again_finds_the_same_user() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;
        let first = repos.sign_in(&identity("user_1"), None).await;

        let again = repos.sign_in(&identity("user_1"), None).await;

        assert_eq!(again, first);
        assert_eq!(count(&pool, "users").await, 1);
    })
    .await;
}

#[tokio::test]
async fn different_provider_users_are_different_users() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;

        let one = repos.sign_in(&identity("user_1"), None).await;
        let two = repos.sign_in(&identity("user_2"), None).await;

        assert_ne!(one.id, two.id);
    })
    .await;
}

#[tokio::test]
async fn the_same_subject_from_another_provider_is_another_person() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;
        let mut elsewhere = identity("user_1");
        elsewhere.provider = "otheridp".to_string();

        let one = repos.sign_in(&identity("user_1"), None).await;
        let two = repos.sign_in(&elsewhere, None).await;

        assert_ne!(one.id, two.id);
    })
    .await;
}

#[tokio::test]
async fn a_guest_who_signs_in_keeps_their_user_and_so_their_matches() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;
        let guest = repos.guest().await;

        let user = repos.sign_in(&identity("user_1"), Some(guest.id)).await;

        assert_eq!(user, guest);
        assert_eq!(count(&pool, "users").await, 1);
        assert_eq!(count(&pool, "identities").await, 1);
    })
    .await;
}

#[tokio::test]
async fn an_account_that_signs_in_on_another_device_gets_its_own_user_not_the_guest() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;
        let account = repos.sign_in(&identity("user_1"), None).await;
        let new_device_guest = repos.guest().await;

        let user = repos
            .sign_in(&identity("user_1"), Some(new_device_guest.id))
            .await;

        assert_eq!(user, account);
        assert_eq!(
            count(&pool, "identities").await,
            1,
            "the guest must not be linked"
        );
    })
    .await;
}

#[tokio::test]
async fn a_signed_in_users_token_cannot_be_used_to_attach_a_second_identity() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;
        let victim = repos.sign_in(&identity("user_1"), None).await;

        let attacker = repos.sign_in(&identity("user_2"), Some(victim.id)).await;

        assert_ne!(attacker.id, victim.id);
        assert_eq!(count(&pool, "identities").await, 2);
    })
    .await;
}

#[tokio::test]
async fn a_guest_that_no_longer_exists_is_ignored() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;

        let user = repos
            .sign_in(&identity("user_1"), Some(UserId::new()))
            .await;

        assert!(user.handle.as_str().starts_with("Player-"));
    })
    .await;
}

#[tokio::test]
async fn two_first_sign_ins_at_once_make_one_user() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;
        let who = identity("user_1");

        let (a, b) = tokio::join!(repos.sign_in(&who, None), repos.sign_in(&who, None));

        assert_eq!(a.id, b.id);
        assert_eq!(count(&pool, "users").await, 1);
        assert_eq!(count(&pool, "identities").await, 1);
    })
    .await;
}

#[tokio::test]
async fn deleting_an_account_frees_the_handle_and_the_identity_but_keeps_the_row() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;
        let user = repos.sign_in(&identity("user_1"), None).await;

        let deleted = delete_account(&repos.accounts, user.id, t0())
            .await
            .unwrap();

        assert!(deleted);
        assert_eq!(repos.users.get(user.id).await.unwrap(), None);
        assert_eq!(count(&pool, "identities").await, 0);
        assert_eq!(
            count(&pool, "users").await,
            1,
            "finished matches still point at it"
        );
        let handle: String = sqlx::query_scalar("SELECT handle FROM users")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert!(handle.starts_with("deleted-"), "{handle}");
        assert_ne!(handle, user.handle.as_str());
    })
    .await;
}

#[tokio::test]
async fn deleting_twice_or_deleting_nobody_reports_nothing_to_delete() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;
        let user = repos.sign_in(&identity("user_1"), None).await;
        assert!(delete_account(&repos.accounts, user.id, t0())
            .await
            .unwrap());

        assert!(!delete_account(&repos.accounts, user.id, t0())
            .await
            .unwrap());
        assert!(!delete_account(&repos.accounts, UserId::new(), t0())
            .await
            .unwrap());
    })
    .await;
}

#[tokio::test]
async fn signing_in_after_deleting_starts_a_fresh_account() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;
        let old = repos.sign_in(&identity("user_1"), None).await;
        delete_account(&repos.accounts, old.id, t0()).await.unwrap();

        let fresh = repos.sign_in(&identity("user_1"), None).await;

        assert_ne!(fresh.id, old.id);
    })
    .await;
}

#[tokio::test]
async fn a_deleted_user_cannot_be_adopted_by_someone_else_with_a_leftover_token() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;
        let old = repos.guest().await;
        delete_account(&repos.accounts, old.id, t0()).await.unwrap();

        let user = repos.sign_in(&identity("user_2"), Some(old.id)).await;

        assert_ne!(user.id, old.id);
    })
    .await;
}

#[tokio::test]
async fn linking_waits_for_a_delete_in_flight_and_is_then_refused() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;
        let guest = repos.guest().await;
        // A delete that has run but not yet committed.
        let mut delete = pool.begin().await.unwrap();
        sqlx::query("UPDATE users SET handle = 'deleted-x', deleted_at = $2 WHERE id = $1")
            .bind(guest.id.as_uuid())
            .bind(t0())
            .execute(&mut *delete)
            .await
            .unwrap();
        let accounts = repos.accounts.clone();
        let who = identity("user_1");
        let link = tokio::spawn(async move { accounts.link(guest.id, &who, t0()).await });
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        delete.commit().await.unwrap();

        let result = link.await.unwrap();
        assert!(matches!(result, Err(StoreError::UserMissing)), "{result:?}");
        assert_eq!(
            count(&pool, "identities").await,
            0,
            "no identity may point at a deleted user"
        );
    })
    .await;
}

#[tokio::test]
async fn linking_a_user_that_never_existed_is_refused() {
    with_db(|pool| async move {
        let repos = repos(&pool).await;

        let result = repos
            .accounts
            .link(UserId::new(), &identity("user_1"), t0())
            .await;

        assert!(matches!(result, Err(StoreError::UserMissing)), "{result:?}");
    })
    .await;
}
