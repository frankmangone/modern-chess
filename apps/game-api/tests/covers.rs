//! Putting a game's cover in the store, against a real Postgres (`make db-up`) and a real
//! S3-compatible store (`make storage-up`). Each test uses a bucket of its own.

use std::io::Cursor;

use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use image::{ImageFormat, Rgb, RgbImage};
use modern_chess_game::store::PageRepo;
use modern_chess_game::{GamePage, Slug};
use modern_chess_game_api::covers::{prepare_store, set_cover, CoverError};
use modern_chess_media::{ImageStore, MediaError, StoreConfig};
use modern_chess_testdb::with_db;
use sqlx::PgPool;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn port() -> u16 {
    env_or("IMAGES_PORT", "9000").parse().unwrap()
}

fn store_settings(bucket: &str) -> StoreConfig {
    StoreConfig {
        endpoint: format!("http://127.0.0.1:{}", port()),
        region: "us-east-1".to_string(),
        bucket: bucket.to_string(),
        access_key: env_or("IMAGES_ACCESS_KEY", "modern_chess"),
        secret_key: env_or("IMAGES_SECRET_KEY", "modern_chess_secret"),
        force_path_style: true,
    }
}

struct Scratch {
    bucket: String,
    store: ImageStore,
}

impl Scratch {
    async fn ready() -> Self {
        let bucket = format!("test-{}", uuid::Uuid::new_v4().simple());
        let store = ImageStore::new(&store_settings(&bucket));
        prepare_store(&store).await.unwrap();
        Self { bucket, store }
    }

    async fn clean_up(self) {
        let settings = store_settings(&self.bucket);
        let client = aws_sdk_s3::Client::from_conf(
            aws_sdk_s3::Config::builder()
                .behavior_version(BehaviorVersion::latest())
                .region(Region::new(settings.region))
                .endpoint_url(settings.endpoint)
                .credentials_provider(Credentials::new(
                    settings.access_key,
                    settings.secret_key,
                    None,
                    None,
                    "test",
                ))
                .force_path_style(true)
                .build(),
        );
        if let Ok(listed) = client.list_objects_v2().bucket(&self.bucket).send().await {
            for object in listed.contents() {
                let _ = client
                    .delete_object()
                    .bucket(&self.bucket)
                    .set_key(object.key().map(str::to_string))
                    .send()
                    .await;
            }
        }
        let _ = client.delete_bucket().bucket(&self.bucket).send().await;
    }
}

/// A request without credentials, the way a browser loads an image.
async fn anonymous_get(bucket: &str, key: &str) -> (u16, String, Vec<u8>) {
    let mut stream = TcpStream::connect(("127.0.0.1", port())).await.unwrap();
    let request =
        format!("GET /{bucket}/{key} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&raw[..split]).to_lowercase();
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    (status, head, raw[split + 4..].to_vec())
}

fn png(width: u32, height: u32, shade: u8) -> Vec<u8> {
    let picture = RgbImage::from_fn(width, height, |x, y| {
        Rgb([(x % 255) as u8, (y % 255) as u8, shade])
    });
    let mut bytes = Vec::new();
    picture
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    bytes
}

async fn game_with_page(pool: &PgPool, cover: Option<&str>) -> PageRepo {
    modern_chess_game::migrate(pool).await.unwrap();
    let pages = PageRepo::new(pool.clone());
    pages
        .upsert(&GamePage {
            slug: Slug::parse("chess").unwrap(),
            name: None,
            preview_variant: None,
            tagline: "t".to_string(),
            description: "d".to_string(),
            tags: vec![],
            accent: None,
            cover_image: cover.map(str::to_string),
            skin: None,
            sections: vec![],
            featured: false,
            sort_order: 0,
        })
        .await
        .unwrap();
    pages
}

async fn cover_of(pages: &PageRepo) -> Option<String> {
    pages
        .get(&Slug::parse("chess").unwrap())
        .await
        .unwrap()
        .unwrap()
        .cover_image
}

#[tokio::test]
async fn a_cover_is_processed_stored_and_recorded_under_its_key() {
    with_db(|pool| async move {
        let pages = game_with_page(&pool, None).await;
        let scratch = Scratch::ready().await;

        let key = set_cover(&scratch.store, &pages, "chess", &png(2400, 1350, 40))
            .await
            .unwrap();

        assert!(
            key.starts_with("covers/chess/") && key.ends_with(".jpg"),
            "{key}"
        );
        assert_eq!(cover_of(&pages).await.as_deref(), Some(key.as_str()));
        let (status, head, body) = anonymous_get(&scratch.bucket, &key).await;
        assert_eq!(status, 200, "{head}");
        assert!(head.contains("content-type: image/jpeg"), "{head}");
        let shown = image::load_from_memory(&body).unwrap();
        assert_eq!((shown.width(), shown.height()), (1200, 675));
        scratch.clean_up().await;
    })
    .await;
}

#[tokio::test]
async fn a_new_picture_replaces_the_cover_and_the_old_file_stays_for_pages_still_showing_it() {
    with_db(|pool| async move {
        let pages = game_with_page(&pool, None).await;
        let scratch = Scratch::ready().await;

        let first = set_cover(&scratch.store, &pages, "chess", &png(800, 450, 10))
            .await
            .unwrap();
        let second = set_cover(&scratch.store, &pages, "chess", &png(800, 450, 200))
            .await
            .unwrap();

        assert_ne!(first, second);
        assert_eq!(cover_of(&pages).await.as_deref(), Some(second.as_str()));
        assert_eq!(anonymous_get(&scratch.bucket, &first).await.0, 200);
        scratch.clean_up().await;
    })
    .await;
}

#[tokio::test]
async fn a_game_without_a_page_is_refused_before_anything_is_uploaded() {
    with_db(|pool| async move {
        modern_chess_game::migrate(&pool).await.unwrap();
        let pages = PageRepo::new(pool.clone());
        let scratch = Scratch::ready().await;
        let file = png(300, 200, 90);

        let result = set_cover(&scratch.store, &pages, "chess", &file).await;

        assert!(matches!(result, Err(CoverError::UnknownGame(_))));
        let would_be = modern_chess_media::process_cover(&file)
            .and_then(|cover| modern_chess_media::cover_key("chess", &cover))
            .unwrap();
        assert_eq!(anonymous_get(&scratch.bucket, &would_be).await.0, 404);
        scratch.clean_up().await;
    })
    .await;
}

#[tokio::test]
async fn a_name_that_could_leave_the_covers_folder_names_no_game() {
    with_db(|pool| async move {
        let pages = game_with_page(&pool, None).await;
        let scratch = Scratch::ready().await;

        let result = set_cover(&scratch.store, &pages, "../secrets", &png(100, 100, 1)).await;

        assert!(matches!(result, Err(CoverError::UnknownGame(_))));
        scratch.clean_up().await;
    })
    .await;
}

#[tokio::test]
async fn something_that_is_not_a_picture_changes_nothing() {
    with_db(|pool| async move {
        let pages = game_with_page(&pool, Some("covers/chess/old.jpg")).await;
        let scratch = Scratch::ready().await;

        let result = set_cover(&scratch.store, &pages, "chess", b"not a picture").await;

        assert!(matches!(
            result,
            Err(CoverError::Image(MediaError::UnsupportedFormat))
        ));
        assert_eq!(
            cover_of(&pages).await.as_deref(),
            Some("covers/chess/old.jpg")
        );
        scratch.clean_up().await;
    })
    .await;
}

#[tokio::test]
async fn an_unreachable_store_leaves_the_old_cover_in_place() {
    with_db(|pool| async move {
        let pages = game_with_page(&pool, Some("covers/chess/old.jpg")).await;
        let mut settings = store_settings("anything");
        settings.endpoint = "http://127.0.0.1:1".to_string();
        let store = ImageStore::new(&settings);

        let result = set_cover(&store, &pages, "chess", &png(300, 200, 5)).await;

        assert!(matches!(
            result,
            Err(CoverError::Image(MediaError::Storage(_)))
        ));
        assert_eq!(
            cover_of(&pages).await.as_deref(),
            Some("covers/chess/old.jpg")
        );
    })
    .await;
}
