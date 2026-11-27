//! Drafts saved to an account and shared by a private link, over HTTP against a real Postgres
//! (`make db-up`). The server keeps text and never runs it, so the texts here are anything at all.

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use chrono::{Duration, Utc};
use modern_chess_game::GameStatus;
use modern_chess_game::{router, GameState, MAX_DRAFTS_PER_AUTHOR, MAX_RULES_BYTES, MOVES};
use modern_chess_kernel::{TokenVerifier, UserId};
use modern_chess_testdb::with_db;
use modern_chess_testkeys::TestKeys;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

struct World {
    app: Router,
    keys: TestKeys,
}

async fn world(pool: PgPool) -> World {
    modern_chess_game::migrate(&pool).await.unwrap();
    let keys = TestKeys::generate();
    let verifier = TokenVerifier::from_public_pem(&keys.public_pem).unwrap();
    World {
        app: router(GameState::new(pool.clone(), verifier)),
        keys,
    }
}

struct Person {
    id: UserId,
    token: String,
}

impl World {
    fn person(&self, handle: &str) -> Person {
        let id = UserId::new();
        let token = self
            .keys
            .token_for(id, handle, (Utc::now() + Duration::hours(1)).timestamp());
        Person { id, token }
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

    async fn keep(&self, by: &Person, name: &str, rules: &str) -> Value {
        let (status, body) = self
            .call(
                Method::POST,
                "/drafts",
                Some(&by.token),
                Some(json!({ "name": name, "rules": rules })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body
    }

    async fn save(
        &self,
        by: &Person,
        id: &str,
        version: i64,
        name: &str,
        rules: &str,
    ) -> (StatusCode, Value) {
        self.call(
            Method::PUT,
            &format!("/drafts/{id}"),
            Some(&by.token),
            Some(json!({ "name": name, "rules": rules, "skin": "", "version": version })),
        )
        .await
    }
}

fn id_of(draft: &Value) -> &str {
    draft["id"].as_str().unwrap()
}

#[tokio::test]
async fn a_draft_is_kept_as_written_even_when_it_is_not_a_game() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        let rules = "{ \"name\": \"half a gam";
        let kept = w.keep(&ada, "March", rules).await;
        assert_eq!(
            (kept["version"].as_i64(), kept["rules"].as_str()),
            (Some(1), Some(rules))
        );
        assert_eq!(kept["share_token"], Value::Null);

        let (status, back) = w
            .call(
                Method::GET,
                &format!("/drafts/{}", id_of(&kept)),
                Some(&ada.token),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(back["rules"], rules);
    })
    .await;
}

#[tokio::test]
async fn people_see_only_their_own_drafts_newest_first() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let (ada, grace) = (w.person("Ada"), w.person("Grace"));
        w.keep(&ada, "First", "{}").await;
        w.keep(&grace, "Hers", "{}").await;
        w.keep(&ada, "Second", "{}").await;

        let (status, list) = w.call(Method::GET, "/drafts", Some(&ada.token), None).await;
        assert_eq!(status, StatusCode::OK);
        let names: Vec<_> = list
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["Second", "First"]);
    })
    .await;
}

#[tokio::test]
async fn signed_out_there_is_nothing_to_list_or_keep() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let (status, _) = w.call(Method::GET, "/drafts", None, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let (status, _) = w
            .call(
                Method::POST,
                "/drafts",
                None,
                Some(json!({ "name": "a", "rules": "{}" })),
            )
            .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    })
    .await;
}

#[tokio::test]
async fn someone_elses_draft_is_not_found_by_every_call_and_is_left_alone() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let (ada, grace) = (w.person("Ada"), w.person("Grace"));
        let draft = w.keep(&ada, "Mine", "{}").await;
        let id = id_of(&draft);
        let (_, shared) = w
            .call(
                Method::PUT,
                &format!("/drafts/{id}/share"),
                Some(&ada.token),
                None,
            )
            .await;

        let uri = format!("/drafts/{id}");
        let share = format!("/drafts/{id}/share");
        let theirs = Some(grace.token.as_str());
        for (method, uri, body) in [
            (Method::GET, uri.as_str(), None),
            (
                Method::PUT,
                uri.as_str(),
                Some(json!({ "name": "x", "rules": "{}", "skin": "", "version": 1 })),
            ),
            (Method::DELETE, uri.as_str(), None),
            (Method::PUT, share.as_str(), None),
            (Method::DELETE, share.as_str(), None),
        ] {
            let (status, _) = w.call(method.clone(), uri, theirs, body).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}");
        }

        // Nothing of Ada's changed: same text, same version, and her link still opens.
        let (_, still) = w.call(Method::GET, &uri, Some(&ada.token), None).await;
        assert_eq!(
            (still["name"].as_str(), still["version"].as_i64()),
            (Some("Mine"), Some(1))
        );
        let token = shared["token"].as_str().unwrap();
        let (status, _) = w
            .call(Method::GET, &format!("/shared/{token}"), None, None)
            .await;
        assert_eq!(status, StatusCode::OK);
    })
    .await;
}

#[tokio::test]
async fn an_id_that_is_not_an_id_is_not_found() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        let (status, _) = w
            .call(Method::GET, "/drafts/nope", Some(&ada.token), None)
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    })
    .await;
}

#[tokio::test]
async fn a_save_counts_up_and_one_from_a_stale_version_changes_nothing() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        let draft = w.keep(&ada, "Game", "one").await;
        let id = id_of(&draft);

        let (status, second) = w.save(&ada, id, 1, "Game", "two").await;
        assert_eq!(
            (status, second["version"].as_i64()),
            (StatusCode::OK, Some(2))
        );

        // A second browser still on version 1.
        let (status, refused) = w.save(&ada, id, 1, "Game", "from the other browser").await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(refused["current_version"].as_i64(), Some(2));
        assert_eq!(refused["error"], "This draft was changed somewhere else.");

        let (_, now) = w
            .call(
                Method::GET,
                &format!("/drafts/{id}"),
                Some(&ada.token),
                None,
            )
            .await;
        assert_eq!(
            (now["rules"].as_str(), now["version"].as_i64()),
            (Some("two"), Some(2))
        );
    })
    .await;
}

#[tokio::test]
async fn two_saves_from_the_same_version_let_exactly_one_win() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        let draft = w.keep(&ada, "Game", "one").await;
        let id = id_of(&draft);

        let (a, b) = tokio::join!(
            w.save(&ada, id, 1, "Game", "a"),
            w.save(&ada, id, 1, "Game", "b")
        );
        let mut statuses = [a.0, b.0];
        statuses.sort();
        assert_eq!(statuses, [StatusCode::OK, StatusCode::CONFLICT]);
    })
    .await;
}

#[tokio::test]
async fn what_is_too_big_or_empty_is_refused_in_words() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        for (name, rules) in [
            ("   ", "{}".to_string()),
            ("x", "r".repeat(MAX_RULES_BYTES + 1)),
        ] {
            let (status, body) = w
                .call(
                    Method::POST,
                    "/drafts",
                    Some(&ada.token),
                    Some(json!({ "name": name, "rules": rules })),
                )
                .await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
            assert!(
                body["error"]
                    .as_str()
                    .unwrap()
                    .starts_with("This draft cannot be kept:"),
                "{body}"
            );
        }
        let (status, _) = w
            .call(
                Method::POST,
                "/drafts",
                Some(&ada.token),
                Some(json!({ "name": 3 })),
            )
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    })
    .await;
}

#[tokio::test]
async fn a_request_far_bigger_than_any_draft_is_turned_away() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        let (status, _) = w
            .call(
                Method::POST,
                "/drafts",
                Some(&ada.token),
                Some(json!({ "name": "x", "rules": "r".repeat(600 * 1024) })),
            )
            .await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    })
    .await;
}

#[tokio::test]
async fn there_is_a_limit_to_drafts_and_deleting_one_makes_room() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let (ada, grace) = (w.person("Ada"), w.person("Grace"));
        let mut last = Value::Null;
        for n in 0..MAX_DRAFTS_PER_AUTHOR {
            last = w.keep(&ada, &format!("Draft {n}"), "{}").await;
        }
        let (status, body) = w
            .call(
                Method::POST,
                "/drafts",
                Some(&ada.token),
                Some(json!({ "name": "one more", "rules": "{}" })),
            )
            .await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");

        // The limit is per person, and a place frees up when a draft goes.
        w.keep(&grace, "Hers", "{}").await;
        let (status, _) = w
            .call(
                Method::DELETE,
                &format!("/drafts/{}", id_of(&last)),
                Some(&ada.token),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        w.keep(&ada, "one more", "{}").await;
    })
    .await;
}

#[tokio::test]
async fn a_deleted_draft_is_gone_with_its_link_and_its_history() {
    with_db(|pool| async move {
        let w = world(pool.clone()).await;
        let ada = w.person("Ada");
        let draft = w.keep(&ada, "Gone", "{}").await;
        let id = id_of(&draft);
        let (_, shared) = w
            .call(
                Method::PUT,
                &format!("/drafts/{id}/share"),
                Some(&ada.token),
                None,
            )
            .await;
        let token = shared["token"].as_str().unwrap();

        let (status, _) = w
            .call(
                Method::DELETE,
                &format!("/drafts/{id}"),
                Some(&ada.token),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let (status, _) = w
            .call(
                Method::GET,
                &format!("/drafts/{id}"),
                Some(&ada.token),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = w
            .call(Method::GET, &format!("/shared/{token}"), None, None)
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM user_game_status_history")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(rows, 0);
    })
    .await;
}

#[tokio::test]
async fn a_private_link_opens_the_game_for_anyone_and_says_nothing_of_the_author() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        let draft = w.keep(&ada, "Secret game", "the rules").await;
        let id = id_of(&draft);

        let (status, first) = w
            .call(
                Method::PUT,
                &format!("/drafts/{id}/share"),
                Some(&ada.token),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        let token = first["token"].as_str().unwrap();
        assert_eq!(token.len(), 22);
        // Asking again gives the same link: it is not a new one each time.
        let (_, again) = w
            .call(
                Method::PUT,
                &format!("/drafts/{id}/share"),
                Some(&ada.token),
                None,
            )
            .await;
        assert_eq!(again["token"], first["token"]);

        // No sign-in needed, and the answer carries the game and nothing else.
        let (status, game) = w
            .call(Method::GET, &format!("/shared/{token}"), None, None)
            .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            game,
            json!({ "name": "Secret game", "rules": "the rules", "skin": "" })
        );

        // The author sees the link on the draft too.
        let (_, mine) = w
            .call(
                Method::GET,
                &format!("/drafts/{id}"),
                Some(&ada.token),
                None,
            )
            .await;
        assert_eq!(mine["share_token"].as_str(), Some(token));
    })
    .await;
}

#[tokio::test]
async fn a_link_shows_the_latest_save_and_stops_when_sharing_stops() {
    with_db(|pool| async move {
        let w = world(pool).await;
        let ada = w.person("Ada");
        let draft = w.keep(&ada, "Game", "v1").await;
        let id = id_of(&draft);
        let (_, shared) = w
            .call(
                Method::PUT,
                &format!("/drafts/{id}/share"),
                Some(&ada.token),
                None,
            )
            .await;
        let old = shared["token"].as_str().unwrap().to_string();

        w.save(&ada, id, 1, "Game", "v2").await;
        let (_, game) = w
            .call(Method::GET, &format!("/shared/{old}"), None, None)
            .await;
        assert_eq!(game["rules"], "v2");

        let (status, _) = w
            .call(
                Method::DELETE,
                &format!("/drafts/{id}/share"),
                Some(&ada.token),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let (status, _) = w
            .call(Method::GET, &format!("/shared/{old}"), None, None)
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        // Turning it off twice is fine, and the draft no longer says it is shared.
        let (status, _) = w
            .call(
                Method::DELETE,
                &format!("/drafts/{id}/share"),
                Some(&ada.token),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        let (_, mine) = w
            .call(
                Method::GET,
                &format!("/drafts/{id}"),
                Some(&ada.token),
                None,
            )
            .await;
        assert_eq!(mine["share_token"], Value::Null);

        // A new link is a new secret: the old one stays dead.
        let (_, renewed) = w
            .call(
                Method::PUT,
                &format!("/drafts/{id}/share"),
                Some(&ada.token),
                None,
            )
            .await;
        let new = renewed["token"].as_str().unwrap();
        assert_ne!(new, old);
        let (status, _) = w
            .call(Method::GET, &format!("/shared/{old}"), None, None)
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = w
            .call(Method::GET, &format!("/shared/{new}"), None, None)
            .await;
        assert_eq!(status, StatusCode::OK);
    })
    .await;
}

#[tokio::test]
async fn a_link_that_is_not_one_opens_nothing() {
    with_db(|pool| async move {
        let w = world(pool).await;
        for raw in [
            "nothing",
            "AAAAAAAAAAAAAAAAAAAAAA",
            "%27%20OR%201=1--",
            "a".repeat(200).as_str(),
        ] {
            let (status, _) = w
                .call(Method::GET, &format!("/shared/{raw}"), None, None)
                .await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{raw}");
        }
    })
    .await;
}

#[tokio::test]
async fn creating_a_draft_writes_the_first_line_of_its_history() {
    with_db(|pool| async move {
        let w = world(pool.clone()).await;
        let ada = w.person("Ada");
        let draft = w.keep(&ada, "Game", "{}").await;

        let row: (Option<String>, String, uuid::Uuid) =
            sqlx::query_as("SELECT from_status, to_status, actor_id FROM user_game_status_history WHERE game_id = $1")
                .bind(id_of(&draft).parse::<uuid::Uuid>().unwrap())
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(row, (None, "DRAFT".to_string(), ada.id.as_uuid()));
    })
    .await;
}

const STATES: [GameStatus; 4] = [
    GameStatus::Draft,
    GameStatus::Published,
    GameStatus::Unpublished,
    GameStatus::Removed,
];

#[tokio::test]
async fn the_database_allows_exactly_the_moves_the_state_machine_has() {
    with_db(|pool| async move {
        let w = world(pool.clone()).await;
        let ada = w.person("Ada");
        let draft = w.keep(&ada, "Game", "{}").await;
        let id: uuid::Uuid = id_of(&draft).parse().unwrap();

        for from in STATES {
            for to in STATES {
                if from == to {
                    continue;
                }
                sqlx::query("UPDATE user_games SET status = 'DRAFT' WHERE id = $1")
                    .bind(id)
                    .execute(&pool)
                    .await
                    .ok();
                // Put the row in `from` by force: the trigger is off for this setup only.
                let mut tx = pool.begin().await.unwrap();
                sqlx::query("ALTER TABLE user_games DISABLE TRIGGER user_games_status_move")
                    .execute(&mut *tx)
                    .await
                    .unwrap();
                sqlx::query("UPDATE user_games SET status = $2 WHERE id = $1")
                    .bind(id)
                    .bind(from.as_db())
                    .execute(&mut *tx)
                    .await
                    .unwrap();
                sqlx::query("ALTER TABLE user_games ENABLE TRIGGER user_games_status_move")
                    .execute(&mut *tx)
                    .await
                    .unwrap();
                tx.commit().await.unwrap();

                let done = sqlx::query("UPDATE user_games SET status = $2 WHERE id = $1")
                    .bind(id)
                    .bind(to.as_db())
                    .execute(&pool)
                    .await;
                let legal = MOVES.iter().any(|(a, _, _, b)| *a == from && *b == to);
                assert_eq!(done.is_ok(), legal, "{from:?} to {to:?}");
            }
        }
    })
    .await;
}

#[tokio::test]
async fn a_game_that_is_not_a_draft_cannot_be_changed_shared_or_deleted_and_is_not_listed() {
    with_db(|pool| async move {
        let w = world(pool.clone()).await;
        let ada = w.person("Ada");
        let draft = w.keep(&ada, "Game", "{}").await;
        let id = id_of(&draft);
        let (_, shared) = w
            .call(
                Method::PUT,
                &format!("/drafts/{id}/share"),
                Some(&ada.token),
                None,
            )
            .await;
        let token = shared["token"].as_str().unwrap().to_string();

        // What publishing will one day do, done by hand here.
        sqlx::query("UPDATE user_games SET status = 'PUBLISHED' WHERE id = $1")
            .bind(id.parse::<uuid::Uuid>().unwrap())
            .execute(&pool)
            .await
            .unwrap();

        let (status, body) = w.save(&ada, id, 1, "Changed", "{}").await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["error"], "Only a draft can be changed.");
        for (method, uri) in [
            (Method::PUT, format!("/drafts/{id}/share")),
            (Method::DELETE, format!("/drafts/{id}/share")),
            (Method::DELETE, format!("/drafts/{id}")),
        ] {
            let (status, _) = w.call(method.clone(), &uri, Some(&ada.token), None).await;
            assert_eq!(status, StatusCode::CONFLICT, "{method} {uri}");
        }
        let (_, list) = w.call(Method::GET, "/drafts", Some(&ada.token), None).await;
        assert!(list.as_array().unwrap().is_empty());
        // The old private link no longer opens a game that is not a draft.
        let (status, _) = w
            .call(Method::GET, &format!("/shared/{token}"), None, None)
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    })
    .await;
}
