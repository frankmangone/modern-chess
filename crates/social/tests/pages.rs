//! The real answer to "is this game page open?": the game service's public page, over HTTP, against
//! a stand-in server that counts how often it is asked.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use modern_chess_social::pages::{GamePages, HttpGamePages};
use tokio::net::TcpListener;

async fn game_service(hits: Arc<AtomicUsize>) -> String {
    async fn page(State(hits): State<Arc<AtomicUsize>>, Path(slug): Path<String>) -> StatusCode {
        hits.fetch_add(1, Ordering::SeqCst);
        match slug.as_str() {
            "open-game" => StatusCode::OK,
            "broken-game" => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::NOT_FOUND,
        }
    }
    let app = Router::new()
        .route("/api/game/catalog/{slug}", get(page))
        .with_state(hits);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{address}/api/game/")
}

#[tokio::test]
async fn a_page_is_open_when_the_game_service_serves_it() {
    let hits = Arc::new(AtomicUsize::new(0));
    let pages = HttpGamePages::new(&game_service(hits.clone()).await).unwrap();

    assert!(pages.is_open("open-game").await.unwrap());
    assert!(!pages.is_open("no-such-game").await.unwrap());
}

#[tokio::test]
async fn answers_are_kept_so_the_game_service_is_not_asked_every_time() {
    let hits = Arc::new(AtomicUsize::new(0));
    let pages = HttpGamePages::new(&game_service(hits.clone()).await).unwrap();

    for _ in 0..5 {
        assert!(pages.is_open("open-game").await.unwrap());
        assert!(!pages.is_open("no-such-game").await.unwrap());
    }

    assert_eq!(hits.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_failure_is_an_error_and_is_not_kept() {
    let hits = Arc::new(AtomicUsize::new(0));
    let pages = HttpGamePages::new(&game_service(hits.clone()).await).unwrap();

    assert!(pages.is_open("broken-game").await.is_err());
    assert!(pages.is_open("broken-game").await.is_err());

    assert_eq!(hits.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn nothing_but_a_slug_is_ever_put_in_the_address() {
    let hits = Arc::new(AtomicUsize::new(0));
    let pages = HttpGamePages::new(&game_service(hits.clone()).await).unwrap();

    assert!(!pages.is_open("../open-game").await.unwrap());
    assert!(!pages.is_open("open-game?x=1").await.unwrap());

    assert_eq!(hits.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn a_game_service_that_cannot_be_reached_is_an_error() {
    let pages = HttpGamePages::new("http://127.0.0.1:1/api/game").unwrap();

    assert!(pages.is_open("open-game").await.is_err());
}
