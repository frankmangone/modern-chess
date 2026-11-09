//! The repositories against a real Postgres (`make db-up`), one throwaway database per test.

use chrono::{DateTime, TimeZone, Utc};
use modern_chess_game::store::{CommandRepo, InstanceRepo, PageRepo, SpecRepo, StoreError};
use modern_chess_game::{
    CommandKind, GameInstance, GamePage, InstanceStatus, LobbyError, PageSection, SeatName, Slug,
};
use modern_chess_kernel::{Handle, UserId};
use modern_chess_testdb::with_db;
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::task::JoinSet;

const CHESS: &str = include_str!("../../engine/specs/chess.json");

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap()
}

fn chess_rules() -> Value {
    serde_json::from_str(CHESS).unwrap()
}

fn slug(raw: &str) -> Slug {
    Slug::parse(raw).unwrap()
}

fn seat(name: &str) -> SeatName {
    SeatName::new(name)
}

fn handle(name: &str) -> Handle {
    Handle::parse(name).unwrap()
}

fn page(tagline: &str) -> GamePage {
    GamePage {
        slug: slug("chess"),
        tagline: tagline.to_string(),
        description: "Kings, queens, and a long history.".to_string(),
        tags: vec!["classic".to_string(), "two-player".to_string()],
        accent: Some("#8a5a2b".to_string()),
        cover_image: None,
        skin: Some(json!({ "light": "#eee" })),
        sections: vec![
            PageSection::HowToPlay {
                markdown: "Move pieces.".to_string(),
            },
            PageSection::OpenLobbies,
        ],
        featured: true,
        sort_order: 2,
    }
}

async fn migrated(pool: &PgPool) {
    modern_chess_game::migrate(pool).await.unwrap();
}

async fn stored_lobby(pool: &PgPool) -> GameInstance {
    let spec = SpecRepo::new(pool.clone())
        .upsert(slug("chess"), "Chess".to_string(), chess_rules(), t0())
        .await
        .unwrap();
    let lobby = GameInstance::new_lobby(&spec, t0()).unwrap();
    InstanceRepo::new(pool.clone())
        .create(&lobby)
        .await
        .unwrap();
    lobby
}

#[tokio::test]
async fn game_migrates_its_own_database() {
    with_db(|pool| async move {
        modern_chess_game::migrate(&pool).await.unwrap();
        modern_chess_game::migrate(&pool)
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
                "game_commands",
                "game_instances",
                "game_pages",
                "game_players",
                "game_specs"
            ],
            "only game tables, none of social's"
        );
    })
    .await;
}

#[tokio::test]
async fn spec_upsert_is_idempotent_for_identical_rules() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let specs = SpecRepo::new(pool.clone());

        let first = specs
            .upsert(slug("chess"), "Chess".to_string(), chess_rules(), t0())
            .await
            .unwrap();
        let again = specs
            .upsert(slug("chess"), "Chess".to_string(), chess_rules(), t0())
            .await
            .unwrap();

        assert_eq!(first.id, again.id);
        assert_eq!(again.version, 1);
        let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM game_specs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 1);
    })
    .await;
}

#[tokio::test]
async fn changed_rules_create_a_new_version() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let specs = SpecRepo::new(pool.clone());
        let first = specs
            .upsert(slug("chess"), "Chess".to_string(), chess_rules(), t0())
            .await
            .unwrap();

        let mut revised = chess_rules();
        revised["name"] = json!("Chess, revised");
        let second = specs
            .upsert(slug("chess"), "Chess".to_string(), revised, t0())
            .await
            .unwrap();

        assert_eq!(second.version, 2);
        assert_ne!(first.id, second.id);
        assert!(second.active);
        assert!(
            !specs.get(first.id).await.unwrap().unwrap().active,
            "the old version stays stored for matches already using it, but is no longer active"
        );
        assert_eq!(
            specs.latest(&slug("chess")).await.unwrap().unwrap().id,
            second.id
        );
    })
    .await;
}

#[tokio::test]
async fn spec_upsert_rejects_rules_the_engine_cannot_read() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let specs = SpecRepo::new(pool.clone());

        let result = specs
            .upsert(
                slug("broken"),
                "Broken".to_string(),
                json!({ "name": "nope" }),
                t0(),
            )
            .await;

        assert!(matches!(result, Err(StoreError::Domain(_))));
    })
    .await;
}

#[tokio::test]
async fn page_upsert_replaces_fields_without_a_new_spec_version() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let specs = SpecRepo::new(pool.clone());
        let pages = PageRepo::new(pool.clone());
        specs
            .upsert(slug("chess"), "Chess".to_string(), chess_rules(), t0())
            .await
            .unwrap();

        pages.upsert(&page("The classic")).await.unwrap();
        let mut edited = page("Still the classic");
        edited.sections = vec![PageSection::Preview];
        edited.accent = None;
        pages.upsert(&edited).await.unwrap();

        assert_eq!(
            pages.get(&slug("chess")).await.unwrap(),
            Some(edited.clone())
        );
        assert_eq!(pages.list().await.unwrap(), vec![edited]);
        assert_eq!(
            specs.latest(&slug("chess")).await.unwrap().unwrap().version,
            1
        );
    })
    .await;
}

#[tokio::test]
async fn a_cover_is_set_and_cleared_on_its_own_without_touching_the_rest_of_the_page() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let pages = PageRepo::new(pool.clone());
        let original = page("The classic");
        pages.upsert(&original).await.unwrap();

        let found = pages
            .set_cover_image(&slug("chess"), Some("covers/chess/a.jpg"))
            .await
            .unwrap();

        assert!(found);
        let stored = pages.get(&slug("chess")).await.unwrap().unwrap();
        assert_eq!(stored.cover_image.as_deref(), Some("covers/chess/a.jpg"));
        assert_eq!(
            GamePage {
                cover_image: None,
                ..stored
            },
            original,
            "only the cover may change"
        );

        pages.set_cover_image(&slug("chess"), None).await.unwrap();
        assert_eq!(
            pages
                .get(&slug("chess"))
                .await
                .unwrap()
                .unwrap()
                .cover_image,
            None
        );
    })
    .await;
}

#[tokio::test]
async fn setting_the_cover_of_a_game_without_a_page_says_so() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let pages = PageRepo::new(pool.clone());

        let found = pages
            .set_cover_image(&slug("chess"), Some("covers/chess/a.jpg"))
            .await
            .unwrap();

        assert!(!found);
    })
    .await;
}

#[tokio::test]
async fn a_page_saved_without_a_cover_keeps_the_one_it_has() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let pages = PageRepo::new(pool.clone());
        pages.upsert(&page("Before")).await.unwrap();
        pages
            .set_cover_image(&slug("chess"), Some("covers/chess/a.jpg"))
            .await
            .unwrap();

        // The seed saves every page again at each start, and it knows nothing of covers.
        pages.upsert(&page("After")).await.unwrap();

        let stored = pages.get(&slug("chess")).await.unwrap().unwrap();
        assert_eq!(stored.tagline, "After");
        assert_eq!(stored.cover_image.as_deref(), Some("covers/chess/a.jpg"));
    })
    .await;
}

#[tokio::test]
async fn a_page_saved_with_a_cover_replaces_the_old_one() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let pages = PageRepo::new(pool.clone());
        pages.upsert(&page("Before")).await.unwrap();
        pages
            .set_cover_image(&slug("chess"), Some("covers/chess/a.jpg"))
            .await
            .unwrap();

        let mut with_cover = page("After");
        with_cover.cover_image = Some("covers/chess/b.jpg".to_string());
        pages.upsert(&with_cover).await.unwrap();

        let stored = pages.get(&slug("chess")).await.unwrap().unwrap();
        assert_eq!(stored.cover_image.as_deref(), Some("covers/chess/b.jpg"));
    })
    .await;
}

#[tokio::test]
async fn instance_roundtrips_through_the_database() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let lobby = stored_lobby(&pool).await;
        let instances = InstanceRepo::new(pool.clone());

        let loaded = instances.get(lobby.id()).await.unwrap().unwrap();

        assert_eq!(loaded, lobby);
        assert_eq!(loaded.status(), InstanceStatus::Lobby);
        assert!(instances
            .get(modern_chess_game::InstanceId::new())
            .await
            .unwrap()
            .is_none());
    })
    .await;
}

#[tokio::test]
async fn player_roundtrips_with_its_handle_copy() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let lobby = stored_lobby(&pool).await;
        let instances = InstanceRepo::new(pool.clone());
        let alice = UserId::new();

        let joined = instances
            .join(lobby.id(), alice, handle("Alice"), seat("BLACK"), t0())
            .await
            .unwrap();

        let loaded = instances.get(lobby.id()).await.unwrap().unwrap();
        assert_eq!(loaded, joined);
        let player = &loaded.players()[0];
        assert_eq!(player.user, alice);
        assert_eq!(player.handle, handle("Alice"));
        assert_eq!(player.seat, seat("BLACK"));
        assert_eq!(player.joined_at, t0());
    })
    .await;
}

#[tokio::test]
async fn joining_an_unknown_instance_is_not_found() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let instances = InstanceRepo::new(pool.clone());

        let result = instances
            .join(
                modern_chess_game::InstanceId::new(),
                UserId::new(),
                handle("Alice"),
                seat("WHITE"),
                t0(),
            )
            .await;

        assert!(matches!(result, Err(StoreError::NotFound)));
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_joins_for_one_seat_have_one_winner() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let lobby = stored_lobby(&pool).await;
        let instances = InstanceRepo::new(pool.clone());

        let mut racers = JoinSet::new();
        for n in 0..8 {
            let instances = instances.clone();
            let id = lobby.id();
            racers.spawn(async move {
                instances
                    .join(
                        id,
                        UserId::new(),
                        handle(&format!("Racer{n}")),
                        seat("WHITE"),
                        t0(),
                    )
                    .await
            });
        }

        let mut winners = 0;
        while let Some(result) = racers.join_next().await {
            match result.unwrap() {
                Ok(_) => winners += 1,
                Err(StoreError::Lobby(LobbyError::SeatTaken)) => {}
                Err(other) => panic!("expected SeatTaken, got {other:?}"),
            }
        }

        assert_eq!(winners, 1);
        let loaded = instances.get(lobby.id()).await.unwrap().unwrap();
        assert_eq!(loaded.players().len(), 1);
    })
    .await;
}

/// The race tests around this one can pass by luck of timing; this one cannot. Another
/// transaction holds the instance row and takes WHITE. A join for WHITE must wait for it and
/// then see the seat as taken, not read the old state and fail on the database constraint.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn join_waits_for_the_instance_row_lock_and_then_sees_its_changes() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let lobby = stored_lobby(&pool).await;
        let instances = InstanceRepo::new(pool.clone());
        let id = lobby.id();

        let mut holder = pool.begin().await.unwrap();
        sqlx::query("SELECT 1 FROM game_instances WHERE id = $1 FOR UPDATE")
            .bind(id.as_uuid())
            .fetch_one(&mut *holder)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO game_players (instance_id, seat, user_id, handle, joined_at) \
             VALUES ($1, 'WHITE', $2, 'Holder', $3)",
        )
        .bind(id.as_uuid())
        .bind(UserId::new().as_uuid())
        .bind(t0())
        .execute(&mut *holder)
        .await
        .unwrap();

        let join = tokio::spawn(async move {
            instances
                .join(id, UserId::new(), handle("Alice"), seat("WHITE"), t0())
                .await
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        assert!(!join.is_finished(), "join must wait for the row's holder");

        holder.commit().await.unwrap();
        let result = join.await.unwrap();
        assert!(
            matches!(result, Err(StoreError::Lobby(LobbyError::SeatTaken))),
            "expected SeatTaken, got {result:?}"
        );
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn racing_for_the_last_seat_activates_the_game_once() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let lobby = stored_lobby(&pool).await;
        let instances = InstanceRepo::new(pool.clone());
        instances
            .join(
                lobby.id(),
                UserId::new(),
                handle("Alice"),
                seat("WHITE"),
                t0(),
            )
            .await
            .unwrap();

        let mut racers = JoinSet::new();
        for n in 0..6 {
            let instances = instances.clone();
            let id = lobby.id();
            racers.spawn(async move {
                instances
                    .join(
                        id,
                        UserId::new(),
                        handle(&format!("Racer{n}")),
                        seat("BLACK"),
                        t0(),
                    )
                    .await
            });
        }

        let mut winners = 0;
        while let Some(result) = racers.join_next().await {
            match result.unwrap() {
                Ok(joined) => {
                    winners += 1;
                    assert_eq!(joined.status(), InstanceStatus::Active);
                }
                Err(StoreError::Lobby(LobbyError::NotInLobby | LobbyError::SeatTaken)) => {}
                Err(other) => panic!("unexpected {other:?}"),
            }
        }

        assert_eq!(winners, 1);
        let loaded = instances.get(lobby.id()).await.unwrap().unwrap();
        assert_eq!(loaded.players().len(), 2);
        assert_eq!(loaded.status(), InstanceStatus::Active);
    })
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn commands_get_consecutive_sequence_numbers() {
    with_db(|pool| async move {
        migrated(&pool).await;
        let lobby = stored_lobby(&pool).await;
        let commands = CommandRepo::new(pool.clone());
        let actor = UserId::new();

        let mut writers = JoinSet::new();
        for n in 0..8u8 {
            let commands = commands.clone();
            let id = lobby.id();
            writers.spawn(async move {
                let kind = CommandKind::Select { x: n, y: 1 };
                if n % 2 == 0 {
                    commands.append(id, actor, kind, true, None, t0()).await
                } else {
                    commands
                        .append(id, actor, kind, false, Some("illegal".to_string()), t0())
                        .await
                }
            });
        }
        let mut seqs = Vec::new();
        while let Some(result) = writers.join_next().await {
            seqs.push(result.unwrap().unwrap().seq);
        }
        seqs.sort();
        assert_eq!(seqs, (1..=8).collect::<Vec<u64>>());

        let log = commands.list(lobby.id()).await.unwrap();
        assert_eq!(
            log.iter().map(|c| c.seq).collect::<Vec<_>>(),
            (1..=8).collect::<Vec<u64>>()
        );
        assert!(log.iter().all(|c| c.accepted == c.error.is_none()));
        assert_eq!(log[0].actor, actor);
    })
    .await;
}
