//! The tool that puts a game's cover image in the image store.
//!
//!   covers init                  create the bucket and open the covers folder (development)
//!   covers set <game> <file>     shrink and store the picture, make it the game's cover
//!
//! It reads the same `.env` files as the server, plus the `IMAGES_*` credentials the server
//! itself never has. Run it with `make covers-init` and `make cover GAME=chess FILE=…`.

use std::process::ExitCode;

use modern_chess_game::store::PageRepo;
use modern_chess_game_api::config::{DatabaseConfig, ImageStorageConfig, ImagesConfig};
use modern_chess_game_api::covers::{prepare_store, set_cover, store_config};
use modern_chess_media::{ImageStore, MAX_COVER_BYTES};
use sqlx::postgres::PgPoolOptions;

type Failure = Box<dyn std::error::Error + Send + Sync>;

const USAGE: &str = "usage:\n  covers init\n  covers set <game> <file>";

#[tokio::main]
async fn main() -> ExitCode {
    modern_chess_game_api::load_env();
    match run(std::env::args().skip(1).collect()).await {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Vec<String>) -> Result<String, Failure> {
    let lookup = |key: &str| std::env::var(key).ok();
    let storage = ImageStorageConfig::from_lookup(&lookup)?;
    let store = ImageStore::new(&store_config(&storage));

    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["init"] => {
            prepare_store(&store).await?;
            Ok(format!(
                "bucket {} is ready; covers/ is public",
                storage.bucket
            ))
        }
        ["set", game, path] => {
            let size = std::fs::metadata(path)?.len();
            if size > MAX_COVER_BYTES as u64 {
                return Err(
                    format!("{path} is {size} bytes; the limit is {MAX_COVER_BYTES}").into(),
                );
            }
            let file = std::fs::read(path)?;

            let database = DatabaseConfig::from_lookup(&lookup)?;
            let pool = PgPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(std::time::Duration::from_secs(
                    database.connect_timeout_secs,
                ))
                .connect(&database.url)
                .await?;
            let key = set_cover(&store, &PageRepo::new(pool), game, &file).await?;

            let base = ImagesConfig::from_lookup(&lookup)?.public_url;
            Ok(format!("{game}: {key}\n{base}/{key}"))
        }
        _ => Err(USAGE.into()),
    }
}
