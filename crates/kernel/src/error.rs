use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("invalid handle: {0}")]
    InvalidHandle(String),
    #[error("invalid user id: {0}")]
    InvalidUserId(String),
}
