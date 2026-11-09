use std::time::Duration;

use aws_sdk_s3::config::retry::RetryConfig;
use aws_sdk_s3::config::timeout::TimeoutConfig;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::error::DisplayErrorContext;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use serde_json::json;

use crate::MediaError;

/// Stored images never change under their name, so browsers and caches may keep them.
const CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

/// Where the store is and how to sign in. Plain values: whoever builds it decides where they
/// come from (the apps read them from `IMAGES_*` settings).
#[derive(Clone)]
pub struct StoreConfig {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub secret_key: String,
    /// `true` for MinIO and most self-hosted stores, which address buckets by path.
    pub force_path_style: bool,
}

// The secret key must never reach a log through a stray `{:?}`.
impl std::fmt::Debug for StoreConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoreConfig")
            .field("endpoint", &self.endpoint)
            .field("region", &self.region)
            .field("bucket", &self.bucket)
            .field("access_key", &self.access_key)
            .field("secret_key", &"<hidden>")
            .field("force_path_style", &self.force_path_style)
            .finish()
    }
}

/// One bucket of an S3-compatible store.
pub struct ImageStore {
    client: Client,
    bucket: String,
}

impl ImageStore {
    pub fn new(config: &StoreConfig) -> Self {
        let credentials = Credentials::new(
            config.access_key.clone(),
            config.secret_key.clone(),
            None,
            None,
            "modern-chess-config",
        );
        let settings = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .endpoint_url(config.endpoint.clone())
            .credentials_provider(credentials)
            .force_path_style(config.force_path_style)
            .retry_config(RetryConfig::standard().with_max_attempts(2))
            .timeout_config(
                TimeoutConfig::builder()
                    .connect_timeout(Duration::from_secs(3))
                    .operation_timeout(Duration::from_secs(30))
                    .build(),
            )
            .build();
        Self {
            client: Client::from_conf(settings),
            bucket: config.bucket.clone(),
        }
    }

    /// Creates the bucket if it is not there. For development stores: a production bucket is
    /// created by whoever provisions the infrastructure.
    pub async fn ensure_bucket(&self) -> Result<(), MediaError> {
        match self
            .client
            .create_bucket()
            .bucket(&self.bucket)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(error)
                if error
                    .as_service_error()
                    .is_some_and(|e| e.is_bucket_already_owned_by_you()) =>
            {
                Ok(())
            }
            Err(error) => Err(failure("creating the bucket", &error)),
        }
    }

    /// Lets anyone read objects under these folders (such as `covers/`) and nothing else. It
    /// replaces any policy the bucket had.
    pub async fn allow_public_reads(&self, folders: &[&str]) -> Result<(), MediaError> {
        if !folders.iter().all(|folder| valid_folder(folder)) {
            return Err(MediaError::InvalidFolder);
        }
        self.client
            .put_bucket_policy()
            .bucket(&self.bucket)
            .policy(public_read_policy(&self.bucket, folders))
            .send()
            .await
            .map_err(|error| failure("setting the bucket policy", &error))?;
        Ok(())
    }

    pub async fn put(
        &self,
        key: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<(), MediaError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .cache_control(CACHE_CONTROL)
            .body(ByteStream::from(bytes))
            .send()
            .await
            .map_err(|error| failure("storing the image", &error))?;
        Ok(())
    }
}

/// The bucket policy that makes objects under `folders` readable by anyone.
pub fn public_read_policy(bucket: &str, folders: &[&str]) -> String {
    let resources: Vec<String> = folders
        .iter()
        .map(|folder| format!("arn:aws:s3:::{bucket}/{folder}*"))
        .collect();
    json!({
        "Version": "2012-10-17",
        "Statement": [{
            "Effect": "Allow",
            "Principal": { "AWS": ["*"] },
            "Action": ["s3:GetObject"],
            "Resource": resources,
        }],
    })
    .to_string()
}

fn valid_folder(folder: &str) -> bool {
    folder.len() > 1
        && folder.ends_with('/')
        && !folder.starts_with('/')
        && folder
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'/')
}

fn failure(action: &str, error: &(impl std::error::Error + 'static)) -> MediaError {
    MediaError::Storage(format!("{action}: {}", DisplayErrorContext(error)))
}
