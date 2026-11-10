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
    #[serde(default)]
    pub sections: Vec<PageSection>,
    #[serde(default)]
    pub featured: bool,
    #[serde(default)]
    pub sort_order: i32,
}
