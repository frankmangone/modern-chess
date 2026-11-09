//! Putting a game's cover image in the store and recording where it is. The `covers` command is
//! a thin caller of this; so will be an upload endpoint when players can propose games.

use modern_chess_game::store::{PageRepo, StoreError};
use modern_chess_game::Slug;
use modern_chess_media::{cover_key, process_cover, ImageStore, MediaError, StoreConfig};
use thiserror::Error;

use crate::config::ImageStorageConfig;

/// The folder of the bucket that anyone may read. Everything else stays private.
const PUBLIC_FOLDER: &str = "covers/";

#[derive(Debug, Error)]
pub enum CoverError {
    #[error("no game called {0:?} has a page")]
    UnknownGame(String),
    #[error(transparent)]
    Image(#[from] MediaError),
    #[error("the game's page could not be updated: {0}")]
    Page(#[from] StoreError),
}

/// What the media crate needs to reach the store, from the app's settings.
pub fn store_config(config: &ImageStorageConfig) -> StoreConfig {
    StoreConfig {
        endpoint: config.endpoint.clone(),
        region: config.region.clone(),
        bucket: config.bucket.clone(),
        access_key: config.access_key.clone(),
        secret_key: config.secret_key.clone(),
        force_path_style: config.force_path_style,
    }
}

/// Creates the bucket if needed and opens the covers folder to anonymous reads. For
/// development stores; a production bucket is provisioned with its policy.
pub async fn prepare_store(store: &ImageStore) -> Result<(), MediaError> {
    store.ensure_bucket().await?;
    store.allow_public_reads(&[PUBLIC_FOLDER]).await
}

/// Checks and shrinks `file`, stores it, and makes it the cover of `game`. Returns the key it
/// was stored under.
///
/// The game must have a page, and the picture must pass the checks, before anything is
/// uploaded. The page changes only after the upload worked, so a failure leaves the old cover
/// in place; at worst a finished upload is left unused, which does no harm because it is named
/// by its content.
pub async fn set_cover(
    store: &ImageStore,
    pages: &PageRepo,
    game: &str,
    file: &[u8],
) -> Result<String, CoverError> {
    let unknown = || CoverError::UnknownGame(game.to_string());
    let slug = Slug::parse(game).map_err(|_| unknown())?;
    pages.get(&slug).await?.ok_or_else(unknown)?;

    let cover = process_cover(file)?;
    let key = cover_key(slug.as_str(), &cover)?;
    store.put(&key, cover.bytes, cover.content_type).await?;

    if !pages.set_cover_image(&slug, Some(&key)).await? {
        return Err(unknown());
    }
    Ok(key)
}
