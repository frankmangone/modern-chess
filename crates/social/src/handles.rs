//! What a person may choose as a username (a handle).
//!
//! The kernel's [`Handle`] says what a handle can be at all: 3 to 24 characters of ASCII
//! letters, digits, `_` and `-`. A name that is *chosen* has to meet more: it starts with a
//! letter or a number, it is not one we keep for ourselves, and it does not look like a name we
//! make (`Guest-1234`, `Player-4821`, `deleted-...`), so nobody can pass as a guest, as someone
//! who has left, or as the product.

use modern_chess_kernel::Handle;

const MIN_LEN: usize = 3;
const MAX_LEN: usize = 24;

/// Names kept for the product, compared without regard to letter case.
const RESERVED: &[&str] = &[
    "admin",
    "administrator",
    "anonymous",
    "api",
    "help",
    "hubbable",
    "me",
    "mod",
    "moderator",
    "null",
    "official",
    "root",
    "security",
    "staff",
    "support",
    "system",
    "undefined",
    "you",
];

/// Prefixes of the names we generate.
const GENERATED_PREFIXES: &[&str] = &["guest-", "player-", "deleted-"];

/// Why a name cannot be chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleProblem {
    TooShort,
    TooLong,
    BadCharacters,
    BadStart,
    Reserved,
    Generated,
}

impl HandleProblem {
    /// A short code for the web app to tell the problems apart.
    pub fn code(self) -> &'static str {
        match self {
            Self::TooShort => "too_short",
            Self::TooLong => "too_long",
            Self::BadCharacters => "bad_characters",
            Self::BadStart => "bad_start",
            Self::Reserved => "reserved",
            Self::Generated => "generated",
        }
    }

    /// What to tell the person.
    pub fn message(self) -> &'static str {
        match self {
            Self::TooShort => "Use at least 3 characters.",
            Self::TooLong => "Use at most 24 characters.",
            Self::BadCharacters => "Use only letters, numbers, dashes and underscores.",
            Self::BadStart => "Start with a letter or a number.",
            Self::Reserved => "That name is kept for us. Please choose another.",
            Self::Generated => {
                "Names that start with Guest-, Player- or deleted- are made for you. Please choose another."
            }
        }
    }
}

/// A name a person may choose, kept exactly as they wrote it.
pub fn choose(raw: &str) -> Result<Handle, HandleProblem> {
    let length = raw.chars().count();
    // Characters first: a name with a space in it is not a short name.
    if !raw
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(HandleProblem::BadCharacters);
    }
    if length < MIN_LEN {
        return Err(HandleProblem::TooShort);
    }
    if length > MAX_LEN {
        return Err(HandleProblem::TooLong);
    }
    if !raw.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        return Err(HandleProblem::BadStart);
    }
    let lower = raw.to_ascii_lowercase();
    if RESERVED.contains(&lower.as_str()) {
        return Err(HandleProblem::Reserved);
    }
    if GENERATED_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
    {
        return Err(HandleProblem::Generated);
    }
    Handle::parse(raw).map_err(|_| HandleProblem::BadCharacters)
}
