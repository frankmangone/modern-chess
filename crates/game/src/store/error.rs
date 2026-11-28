use thiserror::Error;

use crate::{DomainError, DraftError, LobbyError, PlayError, Refusal};

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
    #[error(transparent)]
    Draft(#[from] DraftError),
    /// The game's state machine does not allow that move, or not to that person.
    #[error(transparent)]
    Refused(#[from] Refusal),
    /// The engine stopped while reading rules; see `EngineError::Stopped`.
    #[error("the engine stopped: {0}")]
    Engine(String),
    #[error("not found")]
    NotFound,
}
