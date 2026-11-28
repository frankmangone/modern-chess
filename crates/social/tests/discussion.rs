//! Discussion on a game's page, over HTTP and against the store, with a real Postgres
//! (`make db-up`).

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{header, Method, Request, StatusCode};
use axum::Router;
use chrono::Utc;
use modern_chess_kernel::{Handle, UserId};
use modern_chess_social::pages::{GamePages, OpenFuture, PagesError};
use modern_chess_social::post::{Event, Refusal};
use modern_chess_social::store::{
    DiscussionError, DiscussionRepo, Limits, UserRepo, MAX_POSTS_PER_THREAD,
};
use modern_chess_social::{router, SocialState, TokenIssuer, User};
use modern_chess_testdb::with_db;
use modern_chess_testkeys::TestKeys;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

/// A game service that knows which pages are open, and can be made to fail.
#[derive(Default)]
struct FakePages {
    open: Mutex<HashSet<String>>,
    down: AtomicBool,
}

impl FakePages {
    fn open(&self, slug: &str) {
        self.open.lock().unwrap().insert(slug.to_string());
    }

    fn close(&self, slug: &str) {
        self.open.lock().unwrap().remove(slug);
    }
}

impl GamePages for FakePages {
    fn is_open(&self, slug: &str) -> OpenFuture<'_> {
        let slug = slug.to_string();
        Box::pin(async move {
            if self.down.load(Ordering::SeqCst) {
                return Err(PagesError::Unavailable("down".into()));
            }
            Ok(self.open.lock().unwrap().contains(&slug))
        })
    }
}

struct World {
    app: Router,
    pool: PgPool,
    pages: Arc<FakePages>,
    issuer: Arc<TokenIssuer>,
}

const NO_WAIT: Limits = Limits {
    thread_cooldown: Duration::ZERO,
    post_cooldown: Duration::ZERO,
};

impl World {
    async fn new(pool: &PgPool, limits: Limits) -> Self {
        modern_chess_social::migrate(pool).await.unwrap();
        let keys = TestKeys::generate();
        let issuer = Arc::new(TokenIssuer::from_private_pem(&keys.private_pem).unwrap());
        let pages = Arc::new(FakePages::default());
        pages.open("pong-hau-kitchen");
        let state = SocialState::new(pool.clone(), issuer.clone())
            .with_pages(pages.clone())
            .with_limits(limits);
        Self {
            app: router(state),
            pool: pool.clone(),
            pages,
            issuer,
        }
    }

    /// A signed-in account that has chosen its username.
    async fn account(&self, handle: &str) -> Person {
        let mut user = User::new_account(Handle::parse(handle).unwrap(), Utc::now());
        UserRepo::new(self.pool.clone())
            .insert(&user)
            .await
            .unwrap();
        UserRepo::new(self.pool.clone())
            .finish_setup(user.id)
            .await
            .unwrap();
        user.setup_pending = false;
        self.person(user, true)
    }

    /// An account that has not chosen its username yet.
    async fn fresh_account(&self, handle: &str) -> Person {
        let user = User::new_account(Handle::parse(handle).unwrap(), Utc::now());
        UserRepo::new(self.pool.clone())
            .insert(&user)
            .await
            .unwrap();
        self.person(user, true)
    }

    async fn guest(&self, handle: &str) -> Person {
        let user = User::new_guest(Handle::parse(handle).unwrap(), Utc::now());
        UserRepo::new(self.pool.clone())
            .insert(&user)
            .await
            .unwrap();
        self.person(user, false)
    }

    fn person(&self, user: User, account: bool) -> Person {
        let token = self.issuer.issue(&user, account, Utc::now()).unwrap();
        Person { id: user.id, token }
    }

    async fn send(
        &self,
        method: Method,
        uri: &str,
        who: Option<&Person>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(who) = who {
            request = request.header(header::AUTHORIZATION, format!("Bearer {}", who.token));
        }
        let body = match body {
            Some(body) => {
                request = request.header(header::CONTENT_TYPE, "application/json");
                Body::from(body.to_string())
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

    async fn get(&self, uri: &str, who: Option<&Person>) -> (StatusCode, Value) {
        self.send(Method::GET, uri, who, None).await
    }

    async fn start(&self, who: &Person, title: &str, body: &str) -> (StatusCode, Value) {
        self.send(
            Method::POST,
            "/games/pong-hau-kitchen/threads",
            Some(who),
            Some(json!({ "title": title, "body": body })),
        )
        .await
    }

    async fn thread(&self, who: &Person, title: &str, body: &str) -> String {
        let (status, created) = self.start(who, title, body).await;
        assert_eq!(status, StatusCode::CREATED, "{created}");
        created["id"].as_str().unwrap().to_string()
    }

    async fn reply(&self, who: &Person, thread: &str, body: &str) -> (StatusCode, Value) {
        self.send(
            Method::POST,
            &format!("/threads/{thread}/posts"),
            Some(who),
            Some(json!({ "body": body })),
        )
        .await
    }
}

struct Person {
    id: UserId,
    token: String,
}

fn titles(list: &Value) -> Vec<&str> {
    list["threads"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["title"].as_str().unwrap())
        .collect()
}

#[tokio::test]
async fn anyone_can_read_and_an_account_can_start_a_thread() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;

        let id = world
            .thread(&ana, "Which opening?", "I always start left.")
            .await;

        // No token at all: reading is open.
        let (status, list) = world.get("/games/pong-hau-kitchen/threads", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(titles(&list), ["Which opening?"]);
        assert_eq!(list["threads"][0]["author"], "ana");
        assert_eq!(list["threads"][0]["replies"], 0);
        assert!(list["next"].is_null());

        let (status, page) = world.get(&format!("/threads/{id}"), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page["thread"]["title"], "Which opening?");
        assert_eq!(page["posts"][0]["body"], "I always start left.");
        assert_eq!(page["posts"][0]["opening"], true);
        assert_eq!(page["posts"][0]["mine"], false);
    })
    .await;
}

#[tokio::test]
async fn only_a_signed_in_account_with_a_username_may_write() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;
        let id = world.thread(&ana, "Hello there", "First.").await;
        let guest = world.guest("Guest-4821").await;
        let fresh = world.fresh_account("Player-1234").await;

        let (status, _) = world
            .send(
                Method::POST,
                "/games/pong-hau-kitchen/threads",
                None,
                Some(json!({ "title": "No token", "body": "x" })),
            )
            .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let (status, body) = world.start(&guest, "A guest speaks", "Hi").await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "account_required");
        let (status, body) = world.reply(&guest, &id, "Hi").await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "account_required");

        let (status, body) = world.start(&fresh, "Not set up yet", "Hi").await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "setup_pending");
        let (status, body) = world.reply(&fresh, &id, "Hi").await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "setup_pending");

        // Nothing got in.
        let (_, list) = world.get("/games/pong-hau-kitchen/threads", None).await;
        assert_eq!(titles(&list), ["Hello there"]);
    })
    .await;
}

#[tokio::test]
async fn titles_and_words_are_checked() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;

        for (title, body, code) in [
            ("ab", "fine", "title_too_short"),
            (&"t".repeat(121), "fine", "title_too_long"),
            ("A fine title", "   ", "body_empty"),
            ("A fine title", &"x".repeat(5001), "body_too_long"),
            ("A fine title", "sneaky \u{202E} text", "bad_characters"),
        ] {
            let (status, answer) = world.start(&ana, title, body).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{code}");
            assert_eq!(answer["code"], code);
        }
        let (_, list) = world.get("/games/pong-hau-kitchen/threads", None).await;
        assert!(titles(&list).is_empty());
    })
    .await;
}

#[tokio::test]
async fn a_post_is_plain_text_kept_as_written() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;

        let id = world
            .thread(&ana, "Markup", "<b>bold</b>\r\nsee https://example.test")
            .await;

        let (_, page) = world.get(&format!("/threads/{id}"), None).await;
        assert_eq!(
            page["posts"][0]["body"],
            "<b>bold</b>\nsee https://example.test"
        );
    })
    .await;
}

#[tokio::test]
async fn a_page_that_is_not_open_has_no_discussion() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;
        let id = world.thread(&ana, "Still open", "Hello").await;

        // Unknown, ill-formed, and closed after the fact: all "not found", for every route.
        for slug in ["no-such-game", "Bad_Slug"] {
            let (status, _) = world.get(&format!("/games/{slug}/threads"), None).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{slug}");
            let (status, _) = world
                .send(
                    Method::POST,
                    &format!("/games/{slug}/threads"),
                    Some(&ana),
                    Some(json!({ "title": "Nobody home", "body": "x" })),
                )
                .await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{slug}");
        }
        world.pages.close("pong-hau-kitchen");
        let (status, _) = world.get("/games/pong-hau-kitchen/threads", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = world.get(&format!("/threads/{id}"), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = world.reply(&ana, &id, "Anyone?").await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        // It all comes back with the page: nothing was lost.
        world.pages.open("pong-hau-kitchen");
        let (status, list) = world.get("/games/pong-hau-kitchen/threads", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(titles(&list), ["Still open"]);
    })
    .await;
}

#[tokio::test]
async fn when_the_game_service_cannot_say_discussion_waits() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;
        world.pages.down.store(true, Ordering::SeqCst);

        let (status, _) = world.get("/games/pong-hau-kitchen/threads", None).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        let (status, _) = world.start(&ana, "Anyone there?", "x").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);

        world.pages.down.store(false, Ordering::SeqCst);
        let (status, _) = world.get("/games/pong-hau-kitchen/threads", None).await;
        assert_eq!(status, StatusCode::OK);
    })
    .await;
}

#[tokio::test]
async fn without_a_game_service_to_ask_discussion_is_off() {
    with_db(|pool| async move {
        modern_chess_social::migrate(&pool).await.unwrap();
        let keys = TestKeys::generate();
        let issuer = TokenIssuer::from_private_pem(&keys.private_pem).unwrap();
        let app = router(SocialState::new(pool.clone(), Arc::new(issuer)));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/games/pong-hau-kitchen/threads")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    })
    .await;
}

#[tokio::test]
async fn threads_are_listed_by_latest_activity_and_a_reply_brings_one_up() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;
        let bo = world.account("bo_player").await;
        let first = world.thread(&ana, "First thread", "one").await;
        world.thread(&ana, "Second thread", "two").await;
        let (_, list) = world.get("/games/pong-hau-kitchen/threads", None).await;
        assert_eq!(titles(&list), ["Second thread", "First thread"]);

        let (status, _) = world.reply(&bo, &first, "A reply").await;
        assert_eq!(status, StatusCode::CREATED);

        let (_, list) = world.get("/games/pong-hau-kitchen/threads", None).await;
        assert_eq!(titles(&list), ["First thread", "Second thread"]);
        assert_eq!(list["threads"][0]["replies"], 1);
        assert_eq!(list["threads"][1]["replies"], 0);
    })
    .await;
}

#[tokio::test]
async fn a_discussion_belongs_to_its_game() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        world.pages.open("other-game");
        let ana = world.account("ana").await;
        world.thread(&ana, "On pong", "x").await;
        world
            .send(
                Method::POST,
                "/games/other-game/threads",
                Some(&ana),
                Some(json!({ "title": "On the other", "body": "y" })),
            )
            .await;

        let (_, pong) = world.get("/games/pong-hau-kitchen/threads", None).await;
        let (_, other) = world.get("/games/other-game/threads", None).await;

        assert_eq!(titles(&pong), ["On pong"]);
        assert_eq!(titles(&other), ["On the other"]);
    })
    .await;
}

#[tokio::test]
async fn the_thread_list_comes_in_pages_of_twenty() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;
        for n in 1..=25 {
            world.thread(&ana, &format!("Thread {n:02}"), "x").await;
        }

        let (_, first) = world.get("/games/pong-hau-kitchen/threads", None).await;
        assert_eq!(first["threads"].as_array().unwrap().len(), 20);
        assert_eq!(first["threads"][0]["title"], "Thread 25");
        let next = first["next"].as_str().unwrap();

        let (status, second) = world
            .get(
                &format!("/games/pong-hau-kitchen/threads?before={next}"),
                None,
            )
            .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(second["threads"].as_array().unwrap().len(), 5);
        assert_eq!(second["threads"][0]["title"], "Thread 05");
        assert_eq!(second["threads"][4]["title"], "Thread 01");
        assert!(second["next"].is_null());

        let (status, bad) = world
            .get("/games/pong-hau-kitchen/threads?before=nonsense", None)
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(bad["code"], "bad_cursor");
    })
    .await;
}

#[tokio::test]
async fn a_thread_reads_in_the_order_written_fifty_at_a_time() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;
        let id = world.thread(&ana, "Long thread", "post 0").await;
        for n in 1..=54 {
            let (status, _) = world.reply(&ana, &id, &format!("post {n}")).await;
            assert_eq!(status, StatusCode::CREATED);
        }

        let (_, first) = world.get(&format!("/threads/{id}"), None).await;
        let posts = first["posts"].as_array().unwrap();
        assert_eq!(posts.len(), 50);
        assert_eq!(posts[0]["body"], "post 0");
        assert_eq!(posts[49]["body"], "post 49");
        let next = first["next"].as_i64().unwrap();

        let (_, second) = world
            .get(&format!("/threads/{id}?after={next}"), None)
            .await;
        let posts = second["posts"].as_array().unwrap();
        assert_eq!(posts.len(), 5);
        assert_eq!(posts[0]["body"], "post 50");
        assert_eq!(posts[4]["body"], "post 54");
        assert!(second["next"].is_null());
    })
    .await;
}

#[tokio::test]
async fn a_person_posting_too_fast_is_told_to_wait() {
    with_db(|pool| async move {
        let world = World::new(&pool, Limits::default()).await;
        let ana = world.account("ana").await;
        let bo = world.account("bo_player").await;
        let id = world.thread(&ana, "First thread", "x").await;

        // Her next thread, and a reply right after her opening post, both come too soon...
        let (status, body) = world.start(&ana, "Second thread", "x").await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(body["code"], "too_soon");
        let (status, _) = world.reply(&ana, &id, "and another").await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

        // ...someone else is not held back, and then is, for a reply of their own.
        let (status, _) = world.reply(&bo, &id, "Hello").await;
        assert_eq!(status, StatusCode::CREATED);
        let (status, _) = world.reply(&bo, &id, "Hello again").await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        let (_, page) = world.get(&format!("/threads/{id}"), None).await;
        assert_eq!(page["posts"].as_array().unwrap().len(), 2);
    })
    .await;
}

#[tokio::test]
async fn two_requests_at_once_cannot_both_pass_the_limit() {
    with_db(|pool| async move {
        let world = World::new(&pool, Limits::default()).await;
        let ana = world.account("ana").await;
        let repo = DiscussionRepo::new(pool.clone());

        let start = |title: &'static str| {
            let repo = repo.clone();
            let author = ana.id;
            async move {
                repo.start_thread(
                    "pong-hau-kitchen",
                    author,
                    title,
                    "x",
                    Utc::now(),
                    Limits::default(),
                )
                .await
            }
        };
        let (a, b, c) = tokio::join!(
            start("Thread one"),
            start("Thread two"),
            start("Thread three")
        );

        let made = [a, b, c].into_iter().filter(Result::is_ok).count();
        assert_eq!(made, 1);
    })
    .await;
}

#[tokio::test]
async fn a_thread_takes_a_thousand_posts() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;
        let id = world.thread(&ana, "Endless", "post 0").await;
        sqlx::query(
            "INSERT INTO posts (id, thread_id, author_id, opening, body, created_at) \
             SELECT gen_random_uuid(), $1, $2, FALSE, 'filler', now() FROM generate_series(1, $3)",
        )
        .bind(Uuid::parse_str(&id).unwrap())
        .bind(ana.id.as_uuid())
        .bind(MAX_POSTS_PER_THREAD - 1)
        .execute(&pool)
        .await
        .unwrap();

        let (status, body) = world.reply(&ana, &id, "one too many").await;

        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["code"], "thread_full");
    })
    .await;
}

#[tokio::test]
async fn an_author_deletes_their_own_post_and_it_stays_as_a_grey_line() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;
        let bo = world.account("bo_player").await;
        let id = world.thread(&ana, "A thread", "opening").await;
        let (_, reply) = world.reply(&bo, &id, "my words").await;
        let post = reply["id"].as_str().unwrap();

        // Someone else's post is not theirs to delete, and does not even show as existing.
        let (status, _) = world
            .send(Method::DELETE, &format!("/posts/{post}"), Some(&ana), None)
            .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let (status, _) = world
            .send(Method::DELETE, &format!("/posts/{post}"), None, None)
            .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let (status, _) = world
            .send(Method::DELETE, &format!("/posts/{post}"), Some(&bo), None)
            .await;
        assert_eq!(status, StatusCode::NO_CONTENT);

        let (_, page) = world.get(&format!("/threads/{id}"), None).await;
        let line = &page["posts"][1];
        assert_eq!(line["status"], "deleted");
        assert!(line["body"].is_null(), "the words leave with the post");
        assert_eq!(page["posts"][0]["status"], "visible");
        // Not counted among the replies, and gone for good: it cannot be deleted again.
        let (_, list) = world.get("/games/pong-hau-kitchen/threads", None).await;
        assert_eq!(list["threads"][0]["replies"], 0);
        let (status, body) = world
            .send(Method::DELETE, &format!("/posts/{post}"), Some(&bo), None)
            .await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["code"], "not_allowed");

        let history: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT from_status, to_status, actor FROM post_status_history ORDER BY id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            history,
            [("VISIBLE".into(), "DELETED".into(), "AUTHOR".into())]
        );
    })
    .await;
}

#[tokio::test]
async fn the_page_knows_which_posts_are_the_callers() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;
        let bo = world.account("bo_player").await;
        let id = world.thread(&ana, "A thread", "opening").await;
        world.reply(&bo, &id, "reply").await;

        let (_, as_ana) = world.get(&format!("/threads/{id}"), Some(&ana)).await;
        let (_, as_bo) = world.get(&format!("/threads/{id}"), Some(&bo)).await;
        let (_, as_nobody) = world.get(&format!("/threads/{id}"), None).await;

        assert_eq!(as_ana["posts"][0]["mine"], true);
        assert_eq!(as_ana["posts"][1]["mine"], false);
        assert_eq!(as_bo["posts"][0]["mine"], false);
        assert_eq!(as_bo["posts"][1]["mine"], true);
        assert_eq!(as_nobody["posts"][0]["mine"], false);
    })
    .await;
}

#[tokio::test]
async fn staff_hide_a_post_and_the_thread_goes_when_its_opening_post_does() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;
        let bo = world.account("bo_player").await;
        let staff = world.account("keeper").await;
        let repo = DiscussionRepo::new(pool.clone());
        let id = world.thread(&ana, "A thread", "opening").await;
        let (_, reply) = world.reply(&bo, &id, "a reply").await;
        let reply = Uuid::parse_str(reply["id"].as_str().unwrap()).unwrap();
        let (_, page) = world.get(&format!("/threads/{id}"), None).await;
        let opening = Uuid::parse_str(page["posts"][0]["id"].as_str().unwrap()).unwrap();

        // A stranger who is not staff gets "no such post", not a hint that it exists.
        let stranger = repo
            .move_post(reply, Event::Hide, staff.id, false, None, Utc::now())
            .await;
        assert!(matches!(stranger, Err(DiscussionError::NoSuchPost)));

        // Staff hide the reply: a grey line, words gone, and the author cannot delete it away.
        repo.move_post(reply, Event::Hide, staff.id, true, Some("spam"), Utc::now())
            .await
            .unwrap();
        let (_, page) = world.get(&format!("/threads/{id}"), None).await;
        assert_eq!(page["posts"][1]["status"], "hidden");
        assert!(page["posts"][1]["body"].is_null());
        let escape = repo
            .move_post(reply, Event::Delete, bo.id, false, None, Utc::now())
            .await;
        assert!(matches!(
            escape,
            Err(DiscussionError::Refused(Refusal::NotFromHere))
        ));

        // They bring it back.
        repo.move_post(reply, Event::Reinstate, staff.id, true, None, Utc::now())
            .await
            .unwrap();
        let (_, page) = world.get(&format!("/threads/{id}"), None).await;
        assert_eq!(page["posts"][1]["body"], "a reply");

        // Hiding the opening post takes the thread out of the list, closes it to replies, and
        // leaves it reachable by its address as grey lines.
        repo.move_post(opening, Event::Hide, staff.id, true, None, Utc::now())
            .await
            .unwrap();
        let (_, list) = world.get("/games/pong-hau-kitchen/threads", None).await;
        assert!(titles(&list).is_empty());
        let (status, body) = world.reply(&bo, &id, "still here?").await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body["code"], "thread_closed");
        let (status, page) = world.get(&format!("/threads/{id}"), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(page["posts"][0]["status"], "hidden");

        // Staff acting on their own post act as its author, so cannot hide it.
        let own = world.thread(&staff, "Staff thread", "mine").await;
        let (_, page) = world.get(&format!("/threads/{own}"), None).await;
        let own_post = Uuid::parse_str(page["posts"][0]["id"].as_str().unwrap()).unwrap();
        let own_hide = repo
            .move_post(own_post, Event::Hide, staff.id, true, None, Utc::now())
            .await;
        assert!(matches!(
            own_hide,
            Err(DiscussionError::Refused(Refusal::NotYours))
        ));
    })
    .await;
}

#[tokio::test]
async fn the_database_refuses_the_moves_the_machine_refuses() {
    use modern_chess_social::post::{PostStatus, MOVES};
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;
        let id = world.thread(&ana, "A thread", "words").await;
        let (_, page) = world.get(&format!("/threads/{id}"), None).await;
        let post = Uuid::parse_str(page["posts"][0]["id"].as_str().unwrap()).unwrap();

        let all = [PostStatus::Visible, PostStatus::Hidden, PostStatus::Deleted];
        for from in all {
            for to in all {
                // Put the post in `from` by ignoring the trigger, then try the move.
                sqlx::query("ALTER TABLE posts DISABLE TRIGGER posts_status_move")
                    .execute(&pool)
                    .await
                    .unwrap();
                sqlx::query("UPDATE posts SET status = $2 WHERE id = $1")
                    .bind(post)
                    .bind(from.as_db())
                    .execute(&pool)
                    .await
                    .unwrap();
                sqlx::query("ALTER TABLE posts ENABLE TRIGGER posts_status_move")
                    .execute(&pool)
                    .await
                    .unwrap();

                let result = sqlx::query("UPDATE posts SET status = $2 WHERE id = $1")
                    .bind(post)
                    .bind(to.as_db())
                    .execute(&pool)
                    .await;

                let legal = from == to || MOVES.iter().any(|(a, _, _, b)| *a == from && *b == to);
                assert_eq!(result.is_ok(), legal, "{from:?} to {to:?}");
            }
        }
    })
    .await;
}

#[tokio::test]
async fn a_post_cannot_be_edited_even_by_a_bug() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;
        let id = world.thread(&ana, "A thread", "words").await;

        let result = sqlx::query("UPDATE posts SET body = 'changed' WHERE thread_id = $1")
            .bind(Uuid::parse_str(&id).unwrap())
            .execute(&pool)
            .await;

        assert!(result.is_err());
    })
    .await;
}

#[tokio::test]
async fn a_deleted_account_keeps_its_posts_under_the_name_it_was_given() {
    with_db(|pool| async move {
        let world = World::new(&pool, NO_WAIT).await;
        let ana = world.account("ana").await;
        let id = world.thread(&ana, "Old thread", "from before").await;
        sqlx::query(
            "UPDATE users SET handle = 'deleted-a1b2c3d4', deleted_at = now() WHERE id = $1",
        )
        .bind(ana.id.as_uuid())
        .execute(&pool)
        .await
        .unwrap();

        let (_, list) = world.get("/games/pong-hau-kitchen/threads", None).await;
        let (_, page) = world.get(&format!("/threads/{id}"), None).await;

        assert_eq!(list["threads"][0]["author"], "deleted-a1b2c3d4");
        assert_eq!(page["posts"][0]["author"], "deleted-a1b2c3d4");
        assert_eq!(page["posts"][0]["body"], "from before");
    })
    .await;
}
