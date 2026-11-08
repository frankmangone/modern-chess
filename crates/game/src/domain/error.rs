use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("invalid slug: {0}")]
    InvalidSlug(String),
    #[error("the game rules could not be read: {0}")]
    InvalidRules(String),
    #[error("invalid game instance: {0}")]
    InvalidInstance(String),
}

/// Why a player could not take a seat. Each case maps to a clear message for the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum LobbyError {
    #[error("that seat is already taken")]
    SeatTaken,
    #[error("there is no such seat in this game")]
    UnknownSeat,
    #[error("you already have a seat in this game")]
    AlreadySeated,
    #[error("this game is no longer open to join")]
    NotInLobby,
}

/// Why a move could not be recorded on a match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PlayError {
    #[error("the match has not started")]
    NotStarted,
    #[error("the match is over")]
    Over,
}
