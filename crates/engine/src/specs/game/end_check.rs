//! How a game ends: end checks (a test and what follows when it holds) and the score some of
//! them read.

use serde::{Deserialize, Serialize};

use super::test_block::{FilterSpec, TestSpec};

/// Whose point of view an end check is judged from, as `ALLY` and `ENEMY` are seen from "the
/// mover" in a way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Viewpoint {
    /// The player who just played.
    #[default]
    Mover,
    /// The player about to play.
    Next,
}

/// One end check, written `{ "as": ..., "when": TEST, "then": { "outcome": ... } }`. The first
/// check of `end_checks` whose test holds, after a turn has passed, decides.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EndCheckSpec {
    /// Whose point of view `when` is judged from, and who `WIN` and `LOSE` are about.
    #[serde(default, rename = "as", skip_serializing_if = "is_mover")]
    pub viewpoint: Viewpoint,
    pub when: TestSpec,
    pub then: OutcomeSpec,
}

fn is_mover(viewpoint: &Viewpoint) -> bool {
    *viewpoint == Viewpoint::Mover
}

/// What follows when an end check holds, written `{ "outcome": "WIN" }`. Written as structs
/// so that a field written on one is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(
    tag = "outcome",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum OutcomeSpec {
    /// The `as` player wins.
    Win {},
    /// The `as` player is out of the game; the game ends when one player is left, who wins.
    Lose {},
    /// The game ends with no winner.
    Draw {},
    /// The player with the highest `score` wins; a tie is a draw.
    Highest {},
}

/// One term of the game's `score`, summed for each player.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "term", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum ScoreTermSpec {
    /// The player's pieces on the board that pass `where` (every piece without it). The
    /// filter has no `owner`: the pieces are the player's.
    Pieces {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        r#where: Option<FilterSpec>,
    },
    /// The empty places of each orthogonally joined empty region whose neighbours, among the
    /// pieces that pass `where` (every piece without it), belong to the player alone.
    Territory {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        r#where: Option<FilterSpec>,
    },
    /// The player's counter `name`.
    Counter { name: String },
    /// A fixed number, for the named players.
    Points { value: f64, to: Vec<String> },
}
