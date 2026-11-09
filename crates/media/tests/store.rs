//! The image store against a real S3-compatible server (`make storage-up`). Each test works in
//! a bucket of its own and removes it afterwards.

use std::time::Duration;

use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::Client;
use modern_chess_media::{public_read_policy, ImageStore, MediaError, StoreConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const CACHE: &str = "public, max-age=31536000, immutable";

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn endpoint() -> String {
    format!("http://127.0.0.1:{}", env_or("IMAGES_PORT", "9000"))
}

fn config(bucket: &str) -> StoreConfig {
    StoreConfig {
        endpoint: endpoint(),
        region: "us-east-1".to_string(),
        bucket: bucket.to_string(),
        access_key: env_or("IMAGES_ACCESS_KEY", "modern_chess"),
        secret_key: env_or("IMAGES_SECRET_KEY", "modern_chess_secret"),
        force_path_style: true,
    }
}

/// A client of its own, so the tests can look at the bucket without going through the store.
fn raw_client() -> Client {
    let settings = config("unused");
    let conf = aws_sdk_s3::Config::builder()
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
        .build();
    Client::from_conf(conf)
}

struct Scratch {
    bucket: String,
    store: ImageStore,
}

impl Scratch {
    fn new() -> Self {
        let bucket = format!("test-{}", uuid::Uuid::new_v4().simple());
        let store = ImageStore::new(&config(&bucket));
        Self { bucket, store }
    }

    async fn clean_up(self) {
        let client = raw_client();
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

/// A request with no credentials, the way a browser asks for an image.
async fn anonymous_get(bucket: &str, key: &str) -> (u16, String, Vec<u8>) {
    let port: u16 = env_or("IMAGES_PORT", "9000").parse().unwrap();
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let request = format!(
        "GET /{bucket}/{key} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).await.unwrap();

    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
    (status, head, raw[split + 4..].to_vec())
}

#[tokio::test]
async fn the_bucket_is_created_once_and_asking_again_is_fine() {
    let scratch = Scratch::new();

    scratch.store.ensure_bucket().await.unwrap();
    scratch.store.ensure_bucket().await.unwrap();

    let found = raw_client()
        .head_bucket()
        .bucket(&scratch.bucket)
        .send()
        .await;
    assert!(found.is_ok());
    scratch.clean_up().await;
}

#[tokio::test]
async fn a_cover_is_public_and_cacheable_but_nothing_outside_the_public_folder_is() {
    let scratch = Scratch::new();
    scratch.store.ensure_bucket().await.unwrap();
    scratch
        .store
        .allow_public_reads(&["covers/"])
        .await
        .unwrap();

    scratch
        .store
        .put(
            "covers/chess/abc.jpg",
            b"picture bytes".to_vec(),
            "image/jpeg",
        )
        .await
        .unwrap();
    scratch
        .store
        .put(
            "pending/chess/abc.jpg",
            b"not yet approved".to_vec(),
            "image/jpeg",
        )
        .await
        .unwrap();

    let (status, head, body) = anonymous_get(&scratch.bucket, "covers/chess/abc.jpg").await;
    assert_eq!(status, 200, "{head}");
    assert_eq!(body, b"picture bytes");
    let head = head.to_lowercase();
    assert!(head.contains("content-type: image/jpeg"), "{head}");
    assert!(head.contains(&format!("cache-control: {CACHE}")), "{head}");

    let (status, _, _) = anonymous_get(&scratch.bucket, "pending/chess/abc.jpg").await;
    assert_eq!(
        status, 403,
        "only the public folder may be read without credentials"
    );
    scratch.clean_up().await;
}

#[tokio::test]
async fn a_public_folder_name_cannot_open_up_more_than_it_names() {
    for bad in ["", "/", "*", "covers/*", "a b/", "../x/"] {
        let scratch = Scratch::new();
        scratch.store.ensure_bucket().await.unwrap();

        let result = scratch.store.allow_public_reads(&[bad]).await;

        assert!(
            matches!(result, Err(MediaError::InvalidFolder)),
            "{bad:?} must be refused"
        );
        scratch.clean_up().await;
    }
}

#[test]
fn the_policy_allows_reading_objects_under_the_given_folders_and_nothing_else() {
    let policy: serde_json::Value =
        serde_json::from_str(&public_read_policy("games", &["covers/", "banners/"])).unwrap();

    let statement = &policy["Statement"][0];
    assert_eq!(statement["Effect"], "Allow");
    assert_eq!(statement["Action"], serde_json::json!(["s3:GetObject"]));
    assert_eq!(
        statement["Resource"],
        serde_json::json!([
            "arn:aws:s3:::games/covers/*",
            "arn:aws:s3:::games/banners/*"
        ])
    );
}

#[tokio::test]
async fn an_unreachable_store_is_a_storage_error_not_a_hang() {
    let mut settings = config("anything");
    settings.endpoint = "http://127.0.0.1:1".to_string();
    let store = ImageStore::new(&settings);

    let result = tokio::time::timeout(
        Duration::from_secs(30),
        store.put("covers/x/y.jpg", vec![1, 2, 3], "image/jpeg"),
    )
    .await
    .expect("the call must give up by itself");

    assert!(matches!(result, Err(MediaError::Storage(_))));
}

#[tokio::test]
async fn wrong_credentials_fail_without_repeating_the_secret() {
    let mut settings = config("anything");
    settings.secret_key = "this-secret-must-not-appear".to_string();
    let store = ImageStore::new(&settings);

    let Err(error) = store.ensure_bucket().await else {
        panic!("the server must refuse these credentials");
    };

    assert!(matches!(error, MediaError::Storage(_)));
    assert!(!error.to_string().contains("this-secret-must-not-appear"));
}
