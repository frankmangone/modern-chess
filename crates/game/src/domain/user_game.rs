use chrono::{DateTime, Utc};
use modern_chess_kernel::UserId;
use serde::{Deserialize, Serialize};

use super::{DomainError, GameStatus, Listing, Slug, UserGameId};

/// How long a draft's name may be, in characters.
pub const MAX_NAME_CHARS: usize = 80;
/// How much rules text a draft may hold. A spec this big is already far past anything written by
/// hand; the limit keeps a row small and the lookup of a shared link cheap.
pub const MAX_RULES_BYTES: usize = 128 * 1024;
/// How much look text a draft may hold.
pub const MAX_SKIN_BYTES: usize = 32 * 1024;
/// How many drafts one person may keep.
pub const MAX_DRAFTS_PER_AUTHOR: i64 = 30;

/// What a person writes in a draft: a name and two texts. Neither text has to be valid; a draft is
/// work in progress, and the engine in the person's own browser is what judges it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftContent {
    pub name: String,
    pub rules: String,
    pub skin: String,
}

impl DraftContent {
    /// The content, or the first limit it breaks.
    pub fn new(name: String, rules: String, skin: String) -> Result<Self, DomainError> {
        let name = name.trim().to_string();
        if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
            return Err(DomainError::InvalidDraft(format!(
                "the name must be 1 to {MAX_NAME_CHARS} characters"
            )));
        }
        if rules.len() > MAX_RULES_BYTES {
            return Err(DomainError::InvalidDraft(format!(
                "the rules are too long (at most {} KiB)",
                MAX_RULES_BYTES / 1024
            )));
        }
        if skin.len() > MAX_SKIN_BYTES {
            return Err(DomainError::InvalidDraft(format!(
                "the look is too long (at most {} KiB)",
                MAX_SKIN_BYTES / 1024
            )));
        }
        Ok(Self { name, rules, skin })
    }
}

/// A game made by a person, in whatever state of its life.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserGame {
    pub id: UserGameId,
    pub author: UserId,
    pub content: DraftContent,
    pub status: GameStatus,
    /// Counts the saves. A save names the version it started from, so two browsers cannot silently
    /// overwrite each other.
    pub version: i64,
    pub share_token: Option<ShareToken>,
    /// The game's address, made when its first revision is, and never changed after.
    pub slug: Option<Slug>,
    /// What the page says, once the game has been handed in or published.
    pub listing: Option<Listing>,
    /// For a working copy: the published game it is a new revision of.
    pub revises: Option<UserGameId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserGame {
    pub fn new_draft(author: UserId, content: DraftContent, now: DateTime<Utc>) -> Self {
        Self {
            id: UserGameId::new(),
            author,
            content,
            status: GameStatus::Draft,
            version: 1,
            share_token: None,
            slug: None,
            listing: None,
            revises: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// The secret in a private link: 128 random bits, written in URL-safe base64 (22 characters).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShareToken(String);

impl ShareToken {
    pub fn generate() -> Self {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;
        use rand_core::{OsRng, RngCore};

        let mut bytes = [0_u8; 16];
        OsRng.fill_bytes(&mut bytes);
        Self(URL_SAFE_NO_PAD.encode(bytes))
    }

    /// A token as stored or as typed into a URL, or nothing when it cannot be one. Rejecting the
    /// wrong shape early keeps strange input out of the query.
    pub fn parse(raw: &str) -> Option<Self> {
        let shaped = raw.len() == 22
            && raw
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
        shaped.then(|| Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content(name: &str, rules: &str, skin: &str) -> Result<DraftContent, DomainError> {
        DraftContent::new(name.to_string(), rules.to_string(), skin.to_string())
    }

    #[test]
    fn a_draft_may_hold_text_that_is_not_a_game_yet() {
        assert!(content("March", "{ not json", "").is_ok());
    }

    #[test]
    fn the_name_is_trimmed_and_must_have_something_in_it() {
        assert_eq!(content("  March  ", "{}", "").unwrap().name, "March");
        assert!(content("   ", "{}", "").is_err());
        assert!(content(&"x".repeat(MAX_NAME_CHARS + 1), "{}", "").is_err());
        assert!(content(&"é".repeat(MAX_NAME_CHARS), "{}", "").is_ok());
    }

    #[test]
    fn the_texts_have_a_size_limit() {
        assert!(content("a", &"x".repeat(MAX_RULES_BYTES), "").is_ok());
        assert!(content("a", &"x".repeat(MAX_RULES_BYTES + 1), "").is_err());
        assert!(content("a", "{}", &"x".repeat(MAX_SKIN_BYTES + 1)).is_err());
    }

    #[test]
    fn a_new_draft_is_a_draft_at_version_one_and_shared_with_nobody() {
        let game = UserGame::new_draft(UserId::new(), content("a", "{}", "").unwrap(), Utc::now());
        assert_eq!(
            (game.status, game.version, game.share_token),
            (GameStatus::Draft, 1, None)
        );
    }

    #[test]
    fn tokens_are_long_enough_and_different_each_time() {
        let (a, b) = (ShareToken::generate(), ShareToken::generate());
        assert_ne!(a, b);
        assert_eq!(a.as_str().len(), 22);
        assert_eq!(ShareToken::parse(a.as_str()), Some(a));
    }

    #[test]
    fn a_string_of_the_wrong_shape_is_not_a_token() {
        for raw in [
            "",
            "short",
            "a b c d e f g h i j k l",
            "../../etc/passwd/12345",
            &"a".repeat(23),
        ] {
            assert_eq!(ShareToken::parse(raw), None, "{raw}");
        }
    }
}
