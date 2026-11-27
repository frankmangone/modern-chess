use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("invalid slug: {0}")]
    InvalidSlug(String),
    #[error("the game rules could not be read: {0}")]
    InvalidRules(String),
    #[error("invalid game instance: {0}")]
    InvalidInstance(String),
    #[error("this draft cannot be kept: {0}")]
    InvalidDraft(String),
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

/// Why a draft could not be kept or changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DraftError {
    #[error("you have as many drafts as you can keep; delete one first")]
    TooMany,
    /// The draft was saved from somewhere else since the version the caller started from.
    #[error("this draft was changed somewhere else (it is now at version {current})")]
    Stale { current: i64 },
    /// Published games are not edited: a change is a new revision (decision 3).
    #[error("only a draft can be changed")]
    NotADraft,
}
