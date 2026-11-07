use std::fmt;

use serde::{Deserialize, Serialize};

use super::DomainError;

const MAX_LEN: usize = 64;

/// A game's address in URLs: lowercase letters, digits and `-`, 1 to 64 characters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Slug(String);

impl Slug {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let length_ok = (1..=MAX_LEN).contains(&raw.len());
        let chars_ok = raw
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');

        if !length_ok || !chars_ok {
            return Err(DomainError::InvalidSlug(raw.to_string()));
        }
        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Slug {
    type Error = DomainError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        Self::parse(&raw)
    }
}

impl From<Slug> for String {
    fn from(slug: Slug) -> Self {
        slug.0
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
