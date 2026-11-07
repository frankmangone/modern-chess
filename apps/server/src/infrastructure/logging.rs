use tracing_subscriber::EnvFilter;

use crate::config::{LogFormat, LoggingConfig};

pub fn init(config: &LoggingConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let filter = EnvFilter::try_new(config.level.clone())
        .unwrap_or_else(|_| EnvFilter::new("modern_chess_server=info,info"));

    match config.format {
        LogFormat::Plain => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_target(true)
                .compact()
                .try_init()?;
        }
        LogFormat::Json => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_target(true)
                .json()
                .try_init()?;
        }
    }

    Ok(())
}
