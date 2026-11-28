//! What it takes to publish a game, apart from where it is stored: the words of the listing, the
//! licence, the limits a published game keeps to, and how its address is made from its name.
//!
//! Everything here is pure. The moves between states are in `lifecycle`; the transaction that writes
//! a published game is in `store::publish`.

use serde::Deserialize;

use super::{DomainError, Slug};

/// The one licence every published game is shared under (decision 4 of the ownership plan, first
/// version). It is stated at publish time and the author accepts it by naming it: a form that was
/// opened before the licence changed cannot accept the new one by accident. Whether these are the
/// right words is for someone who knows licensing to say before the terms are written.
pub const LICENCE: &str = "CC-BY-SA-4.0";

/// How long a tagline may be, in characters.
pub const MAX_TAGLINE_CHARS: usize = 140;
/// How long a description may be, in characters.
pub const MAX_DESCRIPTION_CHARS: usize = 2000;

/// The most players a published game may have. The catalogue's biggest game has four; the rest of
/// the margin is for games nobody has thought of yet, not for lobbies nobody can fill. Rules text is
/// capped by the draft's own limit, and the board's size by the engine.
pub const MAX_PLAYERS: usize = 8;

/// Where on the front page a person's game stands: after every game of the catalogue.
pub const SORT_ORDER: i32 = 10_000;

/// What the author writes for the game's page, and the licence they accept.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Listing {
    pub tagline: String,
    pub description: String,
    pub licence: String,
}

impl Listing {
    /// The listing with its words trimmed, or the first thing wrong with it, said in words.
    pub fn checked(self) -> Result<Self, DomainError> {
        let tagline = self.tagline.trim().to_string();
        let description = self.description.trim().to_string();
        if tagline.is_empty() || tagline.chars().count() > MAX_TAGLINE_CHARS {
            return Err(refused(format!(
                "the tagline must be 1 to {MAX_TAGLINE_CHARS} characters"
            )));
        }
        if description.is_empty() || description.chars().count() > MAX_DESCRIPTION_CHARS {
            return Err(refused(format!(
                "the description must be 1 to {MAX_DESCRIPTION_CHARS} characters"
            )));
        }
        if self.licence != LICENCE {
            return Err(refused(format!(
                "publishing means accepting the licence {LICENCE}"
            )));
        }
        Ok(Self {
            tagline,
            description,
            licence: self.licence,
        })
    }
}

fn refused(reason: String) -> DomainError {
    DomainError::InvalidDraft(reason)
}

/// Whether a game with this many players may be published.
pub fn check_players(count: usize) -> Result<(), DomainError> {
    if (1..=MAX_PLAYERS).contains(&count) {
        Ok(())
    } else {
        Err(refused(format!(
            "a published game has 1 to {MAX_PLAYERS} players, and this one has {count}"
        )))
    }
}

/// Words that are routes of the site or of this server, so a game cannot be called by one.
const RESERVED: &[&str] = &[
    "admin", "api", "build", "catalog", "drafts", "games", "new", "review", "shared", "staff",
];

pub fn is_reserved(slug: &str) -> bool {
    RESERVED.contains(&slug)
}

/// The address a game's name asks for: its letters and digits, lowercased, with runs of anything
/// else made into one `-`. Names with nothing in them that an address can hold ("囲碁") ask for
/// `game`. The caller makes it unique.
pub fn slug_base(name: &str) -> Result<Slug, DomainError> {
    let mut out = String::new();
    for c in name.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    // Room is left for a number after it: `-2`, `-37`.
    out.truncate(56);
    let out = out.trim_end_matches('-');
    let out = if out.is_empty() || is_reserved(out) {
        "game".to_string()
    } else {
        out.to_string()
    };
    // Only lowercase letters, digits and dashes, 1 to 56 long, so this does not fail.
    Slug::parse(&out)
}

/// The `n`th try at an address: the base itself, then `base-2`, `base-3` and so on.
pub fn slug_try(base: &Slug, n: u32) -> Result<Slug, DomainError> {
    if n <= 1 {
        Ok(base.clone())
    } else {
        // A base of at most 56 and a number is at most 64.
        Slug::parse(&format!("{base}-{n}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listing(tagline: &str, description: &str, licence: &str) -> Listing {
        Listing {
            tagline: tagline.into(),
            description: description.into(),
            licence: licence.into(),
        }
    }

    #[test]
    fn a_listing_is_trimmed_and_needs_words_and_the_licence() {
        let ok = listing("  Two in a row ", "A game.\n", LICENCE)
            .checked()
            .unwrap();
        assert_eq!(
            (ok.tagline.as_str(), ok.description.as_str()),
            ("Two in a row", "A game.")
        );

        for bad in [
            listing("", "A game.", LICENCE),
            listing("   ", "A game.", LICENCE),
            listing(&"x".repeat(MAX_TAGLINE_CHARS + 1), "A game.", LICENCE),
            listing("Tag", "", LICENCE),
            listing("Tag", &"x".repeat(MAX_DESCRIPTION_CHARS + 1), LICENCE),
            listing("Tag", "A game.", ""),
            listing("Tag", "A game.", "CC0-1.0"),
        ] {
            assert!(bad.clone().checked().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_limits_on_length_count_characters_not_bytes() {
        let tagline = "é".repeat(MAX_TAGLINE_CHARS);
        assert!(listing(&tagline, "A game.", LICENCE).checked().is_ok());
    }

    #[test]
    fn a_game_has_between_one_and_the_most_players() {
        assert!(check_players(0).is_err());
        assert!(check_players(1).is_ok());
        assert!(check_players(MAX_PLAYERS).is_ok());
        assert!(check_players(MAX_PLAYERS + 1).is_err());
    }

    #[test]
    fn a_name_becomes_an_address() {
        for (name, slug) in [
            ("Pong Hau K'i", "pong-hau-k-i"),
            ("  Tic-Tac-Toe!! ", "tic-tac-toe"),
            ("Ünïcode 2", "n-code-2"),
            ("囲碁", "game"),
            ("---", "game"),
            ("New", "game"),
            ("Review", "game"),
        ] {
            assert_eq!(slug_base(name).unwrap().as_str(), slug, "{name:?}");
        }
    }

    #[test]
    fn an_address_from_a_long_name_leaves_room_for_a_number() {
        let base = slug_base(&"a".repeat(200)).unwrap();
        assert_eq!(base.as_str().len(), 56);
        assert_eq!(slug_try(&base, 1).unwrap(), base);
        assert_eq!(slug_try(&base, 12).unwrap().as_str().len(), 59);
    }
}
