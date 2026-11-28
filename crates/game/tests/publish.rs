//! Handing a game in, review, and what publishing writes, over HTTP against a real Postgres
//! (`make db-up`). The rules are a real catalogue game, since publishing has the engine read them.

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use chrono::{Duration, Utc};
use modern_chess_game::store::{PageRepo, SpecRepo};
use modern_chess_game::{router, seed, GameState, Staff, LICENCE};
use modern_chess_kernel::{TokenVerifier, UserId};
use modern_chess_testdb::with_db;
use modern_chess_testkeys::TestKeys;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

const RULES: &str = include_str!("../../engine/specs/pong_hau_ki.json");
const OTHER_RULES: &str = include_str!("../../engine/specs/pong_hau_ki_fan.json");

struct World {
    app: Router,
    state: GameState,
    keys: TestKeys,
    root: Person,
}

#[derive(Clone)]
struct Person {
    id: UserId,
    token: String,
}

async fn world(pool: PgPool) -> World {
    modern_chess_game::migrate(&pool).await.unwrap();
    let keys = TestKeys::generate();
    let verifier = TokenVerifier::from_public_pem(&keys.public_pem).unwrap();
    let root = person_with(&keys, "Root", true);
    let state = GameState::new(pool.clone(), verifier).with_staff(Staff::new([root.id]));
    World {
        app: router(state.clone()),
        state,
        keys,
        root,
    }
}

fn person_with(keys: &TestKeys, handle: &str, account: bool) -> Person {
    let id = UserId::new();
    let exp = (Utc::now() + Duration::hours(1)).timestamp();
    let token = if account {
        keys.account_token_for(id, handle, exp)
    } else {
        keys.token_for(id, handle, exp)
    };
    Person { id, token }
}

fn listing() -> Value {
    json!({ "tagline": "Two in a row", "description": "A game for two.", "licence": LICENCE })
}

impl World {
    fn person(&self, handle: &str) -> Person {
        person_with(&self.keys, handle, true)
    }

    async fn call(
        &self,
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
        let response = self
            .app
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

    /// A draft of the real game, kept under `name`. Returns its id.
    async fn draft(&self, by: &Person, name: &str, rules: &str) -> String {
        let (status, body) = self
            .call(
                Method::POST,
                "/drafts",
                Some(&by.token),
                Some(json!({ "name": name, "rules": rules })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["id"].as_str().unwrap().to_string()
    }

    async fn publish(&self, by: &Person, id: &str, version: i64) -> (StatusCode, Value) {
        let mut body = listing();
        body["version"] = json!(version);
        self.call(
            Method::POST,
            &format!("/drafts/{id}/publish"),
            Some(&by.token),
            Some(body),
        )
        .await
    }

    async fn status_of(&self, by: &Person, id: &str) -> Value {
        let (status, body) = self
            .call(Method::GET, &format!("/drafts/{id}"), Some(&by.token), None)
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    }

    async fn approve(&self, id: &str) -> Result<String, modern_chess_game::store::StoreError> {
        let id = modern_chess_game::UserGameId::from_uuid(id.parse().unwrap());
        self.state
            .publisher()
            .approve(id, self.root.id, Utc::now())
            .await
            .map(|slug| slug.as_str().to_string())
    }

    /// Makes the author trusted the way it happens: a first game, approved by staff.
    async fn trusted(&self, handle: &str) -> Person {
        self.first_game(handle).await.0
    }

    /// A new author with a first game, approved by staff and on the hub as `name`: the person and
    /// the game's id.
    async fn first_game(&self, handle: &str) -> (Person, String) {
        let by = self.person(handle);
        let id = self.draft(&by, "First", RULES).await;
        let (status, _) = self.publish(&by, &id, 1).await;
        assert_eq!(status, StatusCode::ACCEPTED);
        self.approve(&id).await.unwrap();
        (by, id)
    }

    async fn revise(&self, by: &Person, id: &str) -> (StatusCode, Value) {
        self.call(
            Method::POST,
            &format!("/games/{id}/revise"),
            Some(&by.token),
            None,
        )
        .await
    }

    async fn save(&self, by: &Person, id: &str, version: i64, name: &str, rules: &str) {
        let (status, body) = self
            .call(
                Method::PUT,
                &format!("/drafts/{id}"),
                Some(&by.token),
                Some(json!({ "name": name, "rules": rules, "skin": "", "version": version })),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }
}

#[tokio::test]
async fn the_first_game_of_a_new_author_waits_for_review_and_is_not_on_the_hub() {
    with_db(|pool| async move {
        let w = world(pool.clone()).await;
        let ada = w.person("Ada");
        let id = w.draft(&ada, "Tic Tac", RULES).await;

        let (status, body) = w.publish(&ada, &id, 1).await;
        assert_eq!(
            (status, body["status"].as_str()),
            (StatusCode::ACCEPTED, Some("in_review"))
        );

        let game = w.status_of(&ada, &id).await;
        assert_eq!(game["status"], "in_review");
        assert_eq!(game["slug"], Value::Null);
        let (listed, _) = w.call(Method::GET, "/catalog/tic-tac", None, None).await;
        assert_eq!(listed, StatusCode::NOT_FOUND);
        let specs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_specs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(specs, 0);

        // What staff are about to read cannot be changed under them.
        let (status, _) = w
            .call(
                Method::PUT,
                &format!("/drafts/{id}"),
                Some(&ada.token),
                Some(json!({ "name": "Changed", "rules": RULES, "skin": "", "version": 1 })),
            )
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
    })
    .await;
}

#[tokio::test]
async fn an_author_can_withdraw_a_game_from_review_and_it_is_a_draft_again() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        let id = w.draft(&ada, "Tic Tac", RULES).await;
        w.publish(&ada, &id, 1).await;

        let (status, _) = w
            .call(
                Method::POST,
                &format!("/drafts/{id}/withdraw"),
                Some(&ada.token),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(w.status_of(&ada, &id).await["status"], "draft");

        // A draft is not in review, so there is nothing to withdraw.
        let (status, _) = w
            .call(
                Method::POST,
                &format!("/drafts/{id}/withdraw"),
                Some(&ada.token),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
    })
    .await;
}

#[tokio::test]
async fn approving_puts_the_whole_game_on_the_hub_and_it_can_be_played_with_a_friend() {
    with_db(|pool| async move {
        let w = world(pool.clone()).await;
        let ada = w.person("Ada");
        let id = w.draft(&ada, "Tic Tac", RULES).await;
        w.publish(&ada, &id, 1).await;

        let slug = w.approve(&id).await.unwrap();
        assert_eq!(slug, "tic-tac");
        assert_eq!(w.status_of(&ada, &id).await["status"], "published");
        assert_eq!(w.status_of(&ada, &id).await["slug"], "tic-tac");

        let (status, detail) = w.call(Method::GET, "/catalog/tic-tac", None, None).await;
        assert_eq!(status, StatusCode::OK, "{detail}");
        assert_eq!(detail["page"]["tagline"], "Two in a row");
        assert_eq!(detail["page"]["name"], "Tic Tac");
        assert_eq!(detail["spec"]["version"], 1);

        let friend = w.person("Grace");
        let (status, lobby) = w
            .call(Method::POST, "/catalog/tic-tac/lobbies", Some(&friend.token), None)
            .await;
        assert_eq!(status, StatusCode::CREATED, "{lobby}");

        let revision: (i32, String, String, String) = sqlx::query_as(
            "SELECT revision, author_handle, licence, name FROM user_game_revisions WHERE game_id = $1",
        )
        .bind(id.parse::<uuid::Uuid>().unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            revision,
            (1, "Ada".to_string(), LICENCE.to_string(), "Tic Tac".to_string())
        );

        let history: Vec<(Option<String>, String)> = sqlx::query_as(
            "SELECT from_status, to_status FROM user_game_status_history WHERE game_id = $1 ORDER BY id",
        )
        .bind(id.parse::<uuid::Uuid>().unwrap())
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            history,
            vec![
                (None, "DRAFT".to_string()),
                (Some("DRAFT".to_string()), "IN_REVIEW".to_string()),
                (Some("IN_REVIEW".to_string()), "PUBLISHED".to_string()),
            ]
        );
    })
    .await;
}

#[tokio::test]
async fn once_a_game_has_been_approved_the_authors_next_games_publish_at_once() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.trusted("Ada").await;
        let id = w.draft(&ada, "Second", RULES).await;

        let (status, body) = w.publish(&ada, &id, 1).await;
        assert_eq!(
            (status, body["status"].as_str(), body["slug"].as_str()),
            (StatusCode::OK, Some("published"), Some("second"))
        );
        let (status, _) = w.call(Method::GET, "/catalog/second", None, None).await;
        assert_eq!(status, StatusCode::OK);

        // A new author is still reviewed: trust is a person's own.
        let grace = w.person("Grace");
        let theirs = w.draft(&grace, "Third", RULES).await;
        let (status, _) = w.publish(&grace, &theirs, 1).await;
        assert_eq!(status, StatusCode::ACCEPTED);
    })
    .await;
}

#[tokio::test]
async fn staff_can_send_a_game_back_with_a_reason_and_it_is_a_draft_with_the_reason_kept() {
    with_db(|pool| async move {
        let w = world(pool.clone()).await;
        let ada = w.person("Ada");
        let id = w.draft(&ada, "Tic Tac", RULES).await;
        w.publish(&ada, &id, 1).await;
        let game = modern_chess_game::UserGameId::from_uuid(id.parse().unwrap());
        let publisher = w.state.publisher();

        assert!(publisher.reject(game, w.root.id, "   ", Utc::now()).await.is_err());
        publisher
            .reject(game, w.root.id, "Needs a clearer description", Utc::now())
            .await
            .unwrap();

        assert_eq!(w.status_of(&ada, &id).await["status"], "draft");
        let reason: Option<String> = sqlx::query_scalar(
            "SELECT reason FROM user_game_status_history WHERE game_id = $1 AND to_status = 'DRAFT' AND from_status = 'IN_REVIEW'",
        )
        .bind(id.parse::<uuid::Uuid>().unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(reason.as_deref(), Some("Needs a clearer description"));

        // Back in the author's hands: it can be edited, and handed in again.
        let (status, _) = w.publish(&ada, &id, 1).await;
        assert_eq!(status, StatusCode::ACCEPTED);
    })
    .await;
}

#[tokio::test]
async fn only_staff_review_and_nobody_reviews_their_own_game() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        let id = w.draft(&ada, "Tic Tac", RULES).await;
        w.publish(&ada, &id, 1).await;
        let game = modern_chess_game::UserGameId::from_uuid(id.parse().unwrap());
        let publisher = w.state.publisher();

        // A stranger: the game does not exist for them.
        let stranger = UserId::new();
        assert!(matches!(
            publisher.approve(game, stranger, Utc::now()).await,
            Err(modern_chess_game::store::StoreError::NotFound)
        ));
        // The author: it exists, and approving is not theirs to do.
        assert!(matches!(
            publisher.approve(game, ada.id, Utc::now()).await,
            Err(modern_chess_game::store::StoreError::Refused(
                modern_chess_game::Refusal::NotYours
            ))
        ));
        assert_eq!(w.status_of(&ada, &id).await["status"], "in_review");
    })
    .await;
}

#[tokio::test]
async fn staff_who_make_a_game_review_it_as_its_author_and_cannot_approve_it() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let id = w.draft(&w.root, "Mine", RULES).await;
        w.publish(&w.root, &id, 1).await;
        assert!(w.approve(&id).await.is_err());
    })
    .await;
}

#[tokio::test]
async fn rules_the_engine_cannot_read_are_refused_in_words_and_nothing_is_written() {
    with_db(|pool| async move {
        let w = world(pool.clone()).await;
        let ada = w.person("Ada");
        for rules in ["{ \"name\": \"half a gam", "{\"format_version\": 1, \"name\": \"X\"}", "[]"] {
            let id = w.draft(&ada, "Broken", rules).await;
            let (status, body) = w.publish(&ada, &id, 1).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{rules}: {body}");
            assert!(body["error"].as_str().unwrap().starts_with("These rules cannot be published"));
            assert_eq!(w.status_of(&ada, &id).await["status"], "draft");
        }
        let written: i64 = sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM game_specs) + (SELECT count(*) FROM game_pages) + (SELECT count(*) FROM user_game_revisions)",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(written, 0);
    })
    .await;
}

#[tokio::test]
async fn a_listing_without_the_licence_or_words_is_refused() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        let id = w.draft(&ada, "Tic Tac", RULES).await;
        for bad in [
            json!({ "version": 1, "tagline": "", "description": "d", "licence": LICENCE }),
            json!({ "version": 1, "tagline": "t", "description": "", "licence": LICENCE }),
            json!({ "version": 1, "tagline": "t", "description": "d", "licence": "CC0-1.0" }),
        ] {
            let (status, _) = w
                .call(
                    Method::POST,
                    &format!("/drafts/{id}/publish"),
                    Some(&ada.token),
                    Some(bad),
                )
                .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        }
        assert_eq!(w.status_of(&ada, &id).await["status"], "draft");
    })
    .await;
}

#[tokio::test]
async fn a_game_changed_since_the_author_looked_is_not_handed_in() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        let id = w.draft(&ada, "Tic Tac", RULES).await;
        let (status, _) = w
            .call(
                Method::PUT,
                &format!("/drafts/{id}"),
                Some(&ada.token),
                Some(json!({ "name": "Tic Tac", "rules": RULES, "skin": "", "version": 1 })),
            )
            .await;
        assert_eq!(status, StatusCode::OK);

        let (status, body) = w.publish(&ada, &id, 1).await;
        assert_eq!(
            (status, body["current_version"].as_i64()),
            (StatusCode::CONFLICT, Some(2))
        );
        assert_eq!(w.status_of(&ada, &id).await["status"], "draft");
    })
    .await;
}

#[tokio::test]
async fn only_an_accounts_own_draft_can_be_handed_in() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        let id = w.draft(&ada, "Tic Tac", RULES).await;

        let (status, _) = w.publish(&w.person("Grace"), &id, 1).await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let guest = person_with(&w.keys, "Guest-1", false);
        let (status, body) = w.publish(&guest, &id, 1).await;
        assert_eq!(
            (status, body["code"].as_str()),
            (StatusCode::FORBIDDEN, Some("account_required"))
        );

        let (status, _) = w
            .call(
                Method::POST,
                &format!("/drafts/{id}/publish"),
                None,
                Some(json!({})),
            )
            .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn addresses_are_made_from_names_are_unique_and_never_change() {
    with_db(|pool| async move {
        let w = world(pool.clone()).await;
        let ada = w.trusted("Ada").await; // owns "first"
        let mut slugs = Vec::new();
        for name in ["Tic Tac", "tic  tac!", "First", "New"] {
            let id = w.draft(&ada, name, RULES).await;
            let (status, body) = w.publish(&ada, &id, 1).await;
            assert_eq!(status, StatusCode::OK, "{body}");
            slugs.push(body["slug"].as_str().unwrap().to_string());
        }
        assert_eq!(slugs, ["tic-tac", "tic-tac-2", "first-2", "game"]);

        let changed = sqlx::query("UPDATE user_games SET slug = 'other' WHERE slug = 'tic-tac'")
            .execute(&pool)
            .await;
        assert!(changed.is_err(), "the database refuses a new address");
    })
    .await;
}

#[tokio::test]
async fn a_published_revision_cannot_be_changed_or_deleted() {
    with_db(|pool| async move {
        let w = world(pool.clone()).await;
        w.trusted("Ada").await;
        for sql in [
            "UPDATE user_game_revisions SET tagline = 'changed'",
            "DELETE FROM user_game_revisions",
        ] {
            assert!(sqlx::query(sql).execute(&pool).await.is_err(), "{sql}");
        }
        let kept: i64 = sqlx::query_scalar("SELECT count(*) FROM user_game_revisions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(kept, 1);
    })
    .await;
}

#[tokio::test]
async fn the_catalogue_never_takes_a_published_games_address_from_it() {
    with_db(|pool| async move {
        let w = world(pool.clone()).await;
        let ada = w.person("Ada");
        let id = w.draft(&ada, "Tic Tac", RULES).await;
        w.publish(&ada, &id, 1).await;
        w.approve(&id).await.unwrap();

        let page = json!({ "slug": "tic-tac", "tagline": "Ours", "description": "d" }).to_string();
        let report = seed::run_with(
            &SpecRepo::new(pool.clone()),
            &PageRepo::new(pool.clone()),
            &[("tic_tac", RULES)],
            &[("tic-tac", page.as_str())],
            Utc::now(),
        )
        .await
        .unwrap();
        assert_eq!(
            (report.specs, report.pages, report.skipped.len()),
            (0, 0, 2)
        );

        let (_, detail) = w.call(Method::GET, "/catalog/tic-tac", None, None).await;
        assert_eq!(detail["page"]["tagline"], "Two in a row");
        assert_eq!(detail["spec"]["version"], 1);
    })
    .await;
}

#[tokio::test]
async fn editing_a_published_game_starts_a_working_copy_that_is_only_ever_one() {
    with_db(|pool| async move {
        let w = world(pool.clone()).await;
        let (ada, game) = w.first_game("Ada").await;

        let (status, copy) = w.revise(&ada, &game).await;
        assert_eq!(status, StatusCode::CREATED, "{copy}");
        assert_eq!(copy["status"], "draft");
        assert_eq!(copy["revises"], game.as_str());
        assert_eq!(copy["name"], "First");
        assert_eq!(copy["rules"], RULES);
        assert_eq!(copy["listing"]["tagline"], "Two in a row");
        assert_eq!(
            copy["slug"],
            Value::Null,
            "the address stays the original's"
        );

        let (status, again) = w.revise(&ada, &game).await;
        assert_eq!((status, &again["id"]), (StatusCode::OK, &copy["id"]));

        let (_, drafts) = w.call(Method::GET, "/drafts", Some(&ada.token), None).await;
        assert_eq!(drafts.as_array().unwrap().len(), 1);
        let (_, mine) = w
            .call(Method::GET, "/my-games", Some(&ada.token), None)
            .await;
        assert_eq!(mine[0]["working_copy"], copy["id"]);
        assert_eq!(
            (mine[0]["status"].as_str(), mine[0]["revisions"].as_i64()),
            (Some("published"), Some(1))
        );

        // The published game is not touched by what is written in the copy.
        let copy_id = copy["id"].as_str().unwrap();
        w.save(&ada, copy_id, 1, "First, better", OTHER_RULES).await;
        let (_, detail) = w.call(Method::GET, "/catalog/first", None, None).await;
        assert_eq!(
            (
                detail["page"]["name"].as_str(),
                detail["spec"]["version"].as_i64()
            ),
            (Some("First"), Some(1))
        );
    })
    .await;
}

#[tokio::test]
async fn handing_in_a_working_copy_adds_the_next_revision_under_the_same_address() {
    with_db(|pool| async move {
        let w = world(pool.clone()).await;
        let (ada, game) = w.first_game("Ada").await;

        // A match that is already running keeps the rules it started with.
        let friend = w.person("Grace");
        let (status, lobby) = w
            .call(Method::POST, "/catalog/first/lobbies", Some(&friend.token), None)
            .await;
        assert_eq!(status, StatusCode::CREATED, "{lobby}");

        let (_, copy) = w.revise(&ada, &game).await;
        let copy_id = copy["id"].as_str().unwrap().to_string();
        w.save(&ada, &copy_id, 1, "First, better", OTHER_RULES).await;
        let mut body = json!({ "tagline": "Better", "description": "Now with a fan.", "licence": LICENCE });
        body["version"] = json!(2);
        let (status, answer) = w
            .call(Method::POST, &format!("/drafts/{copy_id}/publish"), Some(&ada.token), Some(body))
            .await;
        assert_eq!((status, answer["slug"].as_str()), (StatusCode::OK, Some("first")), "{answer}");

        // The hub shows the new revision, at the same address.
        let (_, detail) = w.call(Method::GET, "/catalog/first", None, None).await;
        assert_eq!(detail["page"]["name"], "First, better");
        assert_eq!(detail["page"]["tagline"], "Better");
        assert_eq!(detail["spec"]["version"], 2);

        // The running lobby is still on version 1.
        let on: i32 = sqlx::query_scalar("SELECT spec_version FROM game_instances WHERE id = $1")
            .bind(lobby["id"].as_str().unwrap().parse::<uuid::Uuid>().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(on, 1);

        // Both revisions are kept, the first as it was.
        let revisions: Vec<(i32, String, String)> = sqlx::query_as(
            "SELECT revision, name, tagline FROM user_game_revisions WHERE game_id = $1 ORDER BY revision",
        )
        .bind(game.parse::<uuid::Uuid>().unwrap())
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            revisions,
            vec![
                (1, "First".to_string(), "Two in a row".to_string()),
                (2, "First, better".to_string(), "Better".to_string()),
            ]
        );

        // The working copy is gone; the game holds the latest content and stays published.
        let (status, _) = w
            .call(Method::GET, &format!("/drafts/{copy_id}"), Some(&ada.token), None)
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (_, mine) = w.call(Method::GET, "/my-games", Some(&ada.token), None).await;
        assert_eq!(
            (mine[0]["status"].as_str(), mine[0]["revisions"].as_i64(), &mine[0]["working_copy"], mine[0]["name"].as_str()),
            (Some("published"), Some(2), &Value::Null, Some("First, better"))
        );

        // And it can be changed again.
        let (status, _) = w.revise(&ada, &game).await;
        assert_eq!(status, StatusCode::CREATED);
    })
    .await;
}

#[tokio::test]
async fn only_a_published_game_of_your_own_can_be_revised() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        let draft = w.draft(&ada, "Tic Tac", RULES).await;

        // A draft is edited in place; a game in review is not edited at all.
        assert_eq!(w.revise(&ada, &draft).await.0, StatusCode::CONFLICT);
        w.publish(&ada, &draft, 1).await;
        assert_eq!(w.revise(&ada, &draft).await.0, StatusCode::CONFLICT);
        w.approve(&draft).await.unwrap();

        assert_eq!(
            w.revise(&w.person("Grace"), &draft).await.0,
            StatusCode::NOT_FOUND
        );
        let guest = person_with(&w.keys, "Guest-1", false);
        assert_eq!(w.revise(&guest, &draft).await.0, StatusCode::FORBIDDEN);
        assert_eq!(w.revise(&ada, &draft).await.0, StatusCode::CREATED);
    })
    .await;
}

#[tokio::test]
async fn a_revision_of_a_game_that_was_taken_off_the_hub_is_not_published() {
    with_db(|pool| async move {
        let w = world(pool.clone()).await;
        let (ada, game) = w.first_game("Ada").await;
        let (_, copy) = w.revise(&ada, &game).await;
        let copy_id = copy["id"].as_str().unwrap();

        sqlx::query("UPDATE user_games SET status = 'UNPUBLISHED' WHERE id = $1")
            .bind(game.parse::<uuid::Uuid>().unwrap())
            .execute(&pool)
            .await
            .unwrap();
        let (status, _) = w.publish(&ada, copy_id, 1).await;
        assert_eq!(status, StatusCode::CONFLICT);
        let revisions: i64 = sqlx::query_scalar("SELECT count(*) FROM user_game_revisions")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(revisions, 1);
        assert_eq!(w.status_of(&ada, copy_id).await["status"], "draft");
    })
    .await;
}

#[tokio::test]
async fn my_games_lists_what_is_in_review_and_the_reason_a_draft_was_sent_back_is_shown() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        let id = w.draft(&ada, "Tic Tac", RULES).await;
        w.publish(&ada, &id, 1).await;

        let (_, mine) = w
            .call(Method::GET, "/my-games", Some(&ada.token), None)
            .await;
        assert_eq!(
            (
                mine.as_array().unwrap().len(),
                mine[0]["status"].as_str(),
                mine[0]["slug"].clone()
            ),
            (1, Some("in_review"), Value::Null)
        );
        let (_, drafts) = w.call(Method::GET, "/drafts", Some(&ada.token), None).await;
        assert!(drafts.as_array().unwrap().is_empty());

        let game = modern_chess_game::UserGameId::from_uuid(id.parse().unwrap());
        w.state
            .publisher()
            .reject(game, w.root.id, "Needs a clearer description", Utc::now())
            .await
            .unwrap();
        let (_, drafts) = w.call(Method::GET, "/drafts", Some(&ada.token), None).await;
        assert_eq!(drafts[0]["rejection"], "Needs a clearer description");
        assert_eq!(
            w.status_of(&ada, &id).await["rejection"],
            "Needs a clearer description"
        );
        let (_, mine) = w
            .call(Method::GET, "/my-games", Some(&ada.token), None)
            .await;
        assert!(mine.as_array().unwrap().is_empty());

        // Handed in again, the old reason is not shown against the new try; withdrawn, nor.
        w.publish(&ada, &id, 1).await;
        w.call(
            Method::POST,
            &format!("/drafts/{id}/withdraw"),
            Some(&ada.token),
            None,
        )
        .await;
        assert_eq!(w.status_of(&ada, &id).await["rejection"], Value::Null);
    })
    .await;
}

#[tokio::test]
async fn working_copies_count_among_the_drafts_a_person_may_keep() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let (ada, game) = w.first_game("Ada").await;
        for n in 0..modern_chess_game::MAX_DRAFTS_PER_AUTHOR {
            w.draft(&ada, &format!("Draft {n}"), RULES).await;
        }
        assert_eq!(w.revise(&ada, &game).await.0, StatusCode::CONFLICT);
    })
    .await;
}
