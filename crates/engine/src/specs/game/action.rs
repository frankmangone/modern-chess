//! Actions and the steps they are made of: choices, tests, effects and `if`.

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use super::test_block::{FilterSpec, PlaceSpec, RegionSpec, TestSpec};

/// The keys that tell the kinds of step apart: a step carries exactly one of them.
const STEP_KEYS: [&str; 4] = ["choose", "test", "effect", "if"];

/// What a player may do on their turn, written `{ "name": ..., "players": [...], "do": [...] }`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSpec {
    /// Unique within the spec.
    pub name: String,
    /// The players who may take it. Empty (the default) means every player.
    #[serde(default)]
    pub players: Vec<String>,
    /// The steps, in order. An empty list is a pass.
    pub r#do: Vec<DoSpec>,
}

/// A step: a choice, a test, an effect or an `if`, told apart by which of `choose`, `test`,
/// `effect` and `if` it carries.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum DoSpec {
    Choose(ChoiceSpec),
    Test(TestSpec),
    Effect(EffectSpec),
    If(IfSpec),
}

impl<'de> Deserialize<'de> for DoSpec {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let fields = Map::<String, Value>::deserialize(deserializer)?;
        let kinds: Vec<&'static str> = STEP_KEYS
            .into_iter()
            .filter(|key| fields.contains_key(*key))
            .collect();
        let [kind] = kinds.as_slice() else {
            return Err(D::Error::custom(step_kind_refusal(&kinds, &fields)));
        };
        let value = Value::Object(fields);
        match *kind {
            "choose" => ChoiceSpec::deserialize(value).map(DoSpec::Choose),
            "test" => TestSpec::deserialize(value).map(DoSpec::Test),
            "effect" => EffectSpec::deserialize(value).map(DoSpec::Effect),
            _ => IfSpec::deserialize(value).map(DoSpec::If),
        }
        .map_err(D::Error::custom)
    }
}

/// Why a step with none, or more than one, of the step keys is refused.
fn step_kind_refusal(kinds: &[&str], fields: &Map<String, Value>) -> String {
    let quoted = |keys: Vec<&str>| {
        keys.iter()
            .map(|key| format!("`{key}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let has = if kinds.is_empty() {
        let written: Vec<&str> = fields.keys().map(String::as_str).collect();
        format!("none (it has {})", quoted(written))
    } else {
        quoted(kinds.to_vec())
    };
    format!(
        "a step carries exactly one of `choose`, `test`, `effect` and `if`, but this one has {has}"
    )
}

/// A question the player answers, written `{ "choose": "PIECE", ... }`. Each choice takes an
/// optional `as` name.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "choose",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum ChoiceSpec {
    /// A place holding a piece that matches `where`. Sets `START`.
    Piece {
        r#where: FilterSpec,
        #[serde(default)]
        r#as: Option<String>,
    },
    /// A place the piece at `START` reaches by its moves. Sets `TARGET`, then runs that move's
    /// own steps.
    Move {
        #[serde(default)]
        r#as: Option<String>,
    },
    /// A place of the region `in`. Sets `TARGET`.
    Place {
        r#in: RegionSpec,
        #[serde(default)]
        r#as: Option<String>,
    },
    /// A piece in the mover's hand whose code matches `where` (`codes` only). Sets `START`, as
    /// "this piece in the hand".
    Hand {
        #[serde(default)]
        r#where: Option<FilterSpec>,
        #[serde(default)]
        r#as: Option<String>,
    },
    /// One of the codes `options`, named `as` for a later `{ "chosen": ... }`.
    Code { r#as: String, options: Vec<String> },
}

/// A change to the position, written `{ "effect": "MOVE", ... }`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "effect",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum EffectSpec {
    /// The piece at `from` goes to `to`; whatever stood on `to` is removed.
    Move { from: PlaceSpec, to: PlaceSpec },
    /// The piece at `at`, if any, leaves the board.
    Remove { at: PlaceSpec },
    /// A new, unmoved piece of `code`, owned by the mover, is put on `at`.
    Place { at: PlaceSpec, code: CodeSpec },
    /// The piece at `at`, if any, now belongs to the mover.
    SetOwner { at: PlaceSpec },
    /// The piece at `at`, if any, becomes `code`.
    SetCode { at: PlaceSpec, code: CodeSpec },
    /// Sets `flag` on the piece at `at`, for `turns` turns, or for good without it.
    SetFlag {
        at: PlaceSpec,
        flag: String,
        #[serde(default)]
        turns: Option<u8>,
    },
    /// Runs `do` once for each place of `in`, with the place `EACH` set to it. No choices.
    ForEach { r#in: RegionSpec, r#do: Vec<DoSpec> },
    /// The turn ends when this way is played, even where a continuation would follow. Written
    /// as a struct so that a field written on it is refused.
    EndTurn {},
}

/// A piece code: written as a string, or `{ "chosen": NAME }` for a `CODE` choice's answer.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum CodeSpec {
    Code(String),
    Chosen { chosen: String },
}

/// `{ "if": TEST, "then": [...], "else": [...] }`: runs `then` when the test holds, otherwise
/// `else`. Either list may be empty or absent.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IfSpec {
    pub r#if: TestSpec,
    #[serde(default)]
    pub then: Vec<DoSpec>,
    #[serde(default)]
    pub r#else: Vec<DoSpec>,
}
