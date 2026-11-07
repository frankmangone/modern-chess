use std::fmt;

use serde::{Deserialize, Serialize};

use crate::DomainError;

const MIN_LEN: usize = 3;
const MAX_LEN: usize = 24;

/// A user's display name: 3 to 24 characters, ASCII letters, digits, `_` and `-`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Handle(String);

impl Handle {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let length_ok = (MIN_LEN..=MAX_LEN).contains(&raw.chars().count());
        let chars_ok = raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');

        if !length_ok || !chars_ok {
            return Err(DomainError::InvalidHandle(raw.to_string()));
        }
        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Handle {
    type Error = DomainError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        Self::parse(&raw)
    }
}

impl From<Handle> for String {
    fn from(handle: Handle) -> Self {
        handle.0
    }
}

impl fmt::Display for Handle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
