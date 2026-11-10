use serde_json::Value;

use super::Slug;

/// One playable variant of a game, such as Go on 13x13.
///
/// Its `slug` is the slug of its rules and of its matches, which is what lets a game be
/// regrouped without touching either. Game slugs and variant slugs are separate namespaces
/// that coincide for a game with a single variant.
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub slug: Slug,
    /// The game this variant belongs to.
    pub game: Slug,
    /// What a picker shows: "13x13".
    pub label: String,
    pub sort_order: i32,
    /// A full skin that replaces the game's, or none to use the game's.
    pub skin: Option<Value>,
}

impl Variant {
    /// The only variant of a game that has no variants of its own: same slug, no own skin.
    pub fn single(game: &Slug, label: &str) -> Self {
        Self {
            slug: game.clone(),
            game: game.clone(),
            label: label.to_string(),
            sort_order: 0,
            skin: None,
        }
    }
}
