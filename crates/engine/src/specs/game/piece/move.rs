use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::shared::ExtendedPosition;
use crate::specs::game::test_block::{ConditionEntry, TestSpec};

use super::action::ActionSpec;
use super::modifier::ModifierSpec;
use super::side_effect::SideEffectSpec;

/// Where a move's step goes: a vector on a grid (`[0, 1]`), or the label of a map's links
/// (`"next"`), which leads to every place the label links to.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StepSpec {
    Vector(ExtendedPosition),
    Label(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MoveSpec {
    pub id: u8,
    pub step: StepSpec,
    pub actions: Vec<ActionSpec>,

    /// Restricts the move to the listed players. Empty (the default) means every player.
    #[serde(default)]
    pub players: Vec<String>,

    /// Tests that must all hold, judged in order.
    #[serde(default)]
    pub conditions: Vec<ConditionEntry>,

    #[serde(default)]
    pub modifiers: Vec<ModifierSpec>,

    #[serde(default)]
    pub side_effects: Vec<SideEffectSpec>,

    #[serde(default)]
    pub repeat: Option<RepeatSpec>,

    /// When `true`, this blueprint contributes to the attack map but generates no legal moves.
    /// Use it to model threat patterns that differ from legal movement (e.g. the Xiangqi
    /// flying-general rule, where the General's threat extends along the full rank/file even
    /// though it can only step one square legally).
    #[serde(default)]
    pub threat_only: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RepeatSpec {
    /// When the slide stops: judged on the next square (`TARGET`) before going on from it, with
    /// `START` the piece's square. Defaults to the next square being occupied.
    #[serde(default)]
    pub until: Option<UntilSpec>,

    #[serde(default)]
    pub times: Option<u8>,

    #[serde(default)]
    #[serde(rename = "loop")]
    pub loop_move: bool,
}

/// A repeat's `until`: a test, or a condition named as before tests (until every spec is
/// rewritten).
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum UntilSpec {
    Test(TestSpec),
    Legacy(String),
}

/// A string is read as a named condition and anything else as a test, so a mistake in a test is
/// reported as the test's own.
impl<'de> Deserialize<'de> for UntilSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Value::deserialize(deserializer)? {
            Value::String(name) => Ok(UntilSpec::Legacy(name)),
            value => TestSpec::deserialize(value)
                .map(UntilSpec::Test)
                .map_err(serde::de::Error::custom),
        }
    }
}
