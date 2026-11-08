use thiserror::Error;

use crate::{DomainError, LobbyError, PlayError};

/// Why a repository call failed. Callers translate these; none of the text is meant for
/// players.
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    /// A stored row, or a value being stored, broke a domain rule.
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error(transparent)]
    Lobby(#[from] LobbyError),
    #[error(transparent)]
    Play(#[from] PlayError),
    #[error("not found")]
    NotFound,
}
