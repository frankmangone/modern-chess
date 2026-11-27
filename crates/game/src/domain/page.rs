use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Slug;

/// One block on a game's page. A game chooses which blocks it shows and in what order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PageSection {
    HowToPlay {
        markdown: String,
    },
    /// A playable demo board.
    Preview,
    /// The lobbies waiting for players.
    OpenLobbies,
}

/// The tunable front page of one game: what it says and how it looks.
///
/// This is data, not code, so a game's page can change without a release. A game appears
/// on the front page if and only if it has one, and at least one variant that can be played.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GamePage {
    pub slug: Slug,
    /// The game's own name ("Go"). Without one, the name of its preview variant's rules is used.
    #[serde(default)]
    pub name: Option<String>,
    /// The variant the "Try it" board uses. Without one, the first variant.
    #[serde(default)]
    pub preview_variant: Option<Slug>,
    pub tagline: String,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// A CSS colour that tints the game's card and page.
    #[serde(default)]
    pub accent: Option<String>,
    #[serde(default)]
    pub cover_image: Option<String>,
    /// The board skin, in the same format the docs board uses.
    #[serde(default)]
    pub skin: Option<Value>,
    /// The colours of the game's own pages, in place of the site's walnut.
    #[serde(default)]
    pub theme: Option<PageTheme>,
    #[serde(default)]
    pub sections: Vec<PageSection>,
    /// The game this one is a variant of ("chess" for Atomic). The front page lists it under that
    /// game rather than on its own, and that game's page lists it among its variants.
    #[serde(default)]
    pub parent: Option<Slug>,
    #[serde(default)]
    pub featured: bool,
    #[serde(default)]
    pub sort_order: i32,
}

/// The colours a game's pages are drawn in. Each one left out keeps the site's own; the web app
/// works out the other shades (outlines, shadows, quieter text) from these.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PageTheme {
    /// The background behind everything.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground: Option<Colour>,
    /// Cards, panels and the text written straight on the ground.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paper: Option<Colour>,
    /// Text on paper, and the outlines.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ink: Option<Colour>,
    /// The one bright colour: buttons, title bars, links on the ground.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent: Option<Colour>,
    /// Small marks: the turn dot, zone stripes and the board's highlights.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<Colour>,
}

/// A colour written `#rrggbb`. Nothing else is accepted, so a theme can only ever be colours.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Colour(String);

impl Colour {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Colour {
    type Error = String;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let hex = raw.strip_prefix('#').unwrap_or("");
        if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(Self(raw.to_ascii_lowercase()))
        } else {
            Err(format!("a theme colour is written #rrggbb, not {raw:?}"))
        }
    }
}

impl From<Colour> for String {
    fn from(colour: Colour) -> Self {
        colour.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_theme_takes_only_hex_colours() {
        let theme: PageTheme = serde_json::from_str(r##"{"ground": "#3B3A37"}"##).unwrap();
        assert_eq!(theme.ground.unwrap().as_str(), "#3b3a37");
        for bad in [
            r#"{"ground": "red"}"#,
            r##"{"ground": "#123"}"##,
            r##"{"ground": "#12345g"}"##,
        ] {
            assert!(serde_json::from_str::<PageTheme>(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_misspelt_colour_name_is_refused() {
        assert!(serde_json::from_str::<PageTheme>(r##"{"backgroud": "#000000"}"##).is_err());
    }
}
