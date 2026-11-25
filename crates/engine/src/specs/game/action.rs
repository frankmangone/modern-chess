//! Actions and the steps they are made of: choices, tests, effects and `if`.

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

use super::bag::BagRef;
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
    /// The stage of `turns.stages` it belongs to: it is offered only there. A stage some action
    /// belongs to offers those actions alone. Without it, the action is offered on an ordinary
    /// turn, and in the stages no action belongs to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
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
    /// A piece held in the bag `in` whose code matches `where` (`codes` only). Sets `START`, as
    /// "this piece in the bag".
    Held {
        r#in: BagRef,
        #[serde(default)]
        r#where: Option<FilterSpec>,
        #[serde(default)]
        r#as: Option<String>,
    },
    /// One of the codes `options`, named `as` for a later `{ "chosen": ... }`.
    Code { r#as: String, options: Vec<String> },
}

/// How many pieces: a number above 0, `ALL`, or what is counted in the position, written
/// `{ "count": { "in": R, "where": F } }` (as a `COUNT` test counts).
#[derive(Clone, Debug, PartialEq)]
pub enum CountSpec {
    Number(u32),
    All,
    Of(Box<CountOfSpec>),
}

/// What a `{ "count": ... }` counts: the pieces that pass `where` (every piece without it) in the
/// places of the region `in`, holders included.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CountOfSpec {
    pub r#in: RegionSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#where: Option<FilterSpec>,
}

impl PartialEq for CountOfSpec {
    fn eq(&self, other: &Self) -> bool {
        serde_json::to_value(self).ok() == serde_json::to_value(other).ok()
    }
}

impl Serialize for CountSpec {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            CountSpec::Number(number) => serializer.serialize_u32(*number),
            CountSpec::All => serializer.serialize_str("ALL"),
            CountSpec::Of(of) => {
                let mut map = serde_json::Map::new();
                map.insert(
                    "count".to_string(),
                    serde_json::to_value(of).map_err(serde::ser::Error::custom)?,
                );
                Value::Object(map).serialize(serializer)
            }
        }
    }
}

impl<'de> Deserialize<'de> for CountSpec {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        const WHAT: &str =
            "a `count` is a whole number above 0, \"ALL\", or { \"count\": { \"in\": ... } }";
        match Value::deserialize(deserializer)? {
            Value::Number(number) => match number.as_u64().and_then(|n| u32::try_from(n).ok()) {
                Some(number) if number > 0 => Ok(CountSpec::Number(number)),
                _ => Err(D::Error::custom(WHAT)),
            },
            Value::String(text) if text == "ALL" => Ok(CountSpec::All),
            Value::Object(mut map) if map.len() == 1 && map.contains_key("count") => {
                let inner = map.remove("count").unwrap_or(Value::Null);
                serde_json::from_value(inner)
                    .map(|of| CountSpec::Of(Box::new(of)))
                    .map_err(D::Error::custom)
            }
            _ => Err(D::Error::custom(WHAT)),
        }
    }
}

/// A change to the position, written `{ "effect": "MOVE", ... }`.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "effect",
    rename_all = "SCREAMING_SNAKE_CASE",
    deny_unknown_fields
)]
pub enum EffectSpec {
    /// The piece at `from` goes to `to`; whatever stood on `to` is removed, and goes where
    /// `removed_to` says, if it does.
    Move {
        from: PlaceSpec,
        to: PlaceSpec,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        removed_to: Option<BagRef>,
        /// Which pieces move, from a holder: `codes` and `owner`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        what: Option<FilterSpec>,
        /// How many move, from a holder: a number, or `ALL`. One when not written.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        count: Option<CountSpec>,
    },
    /// The piece at `at`, if any, leaves the board, for where `removed_to` says if it does.
    Remove {
        at: PlaceSpec,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        removed_to: Option<BagRef>,
        /// Which pieces are removed, from a holder.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        what: Option<FilterSpec>,
        /// How many are removed, from a holder: a number, or `ALL`. One when not written.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        count: Option<CountSpec>,
    },
    /// A new, unmoved piece of `code`, owned by the mover, is put on `at`.
    Place {
        at: PlaceSpec,
        code: CodeSpec,
        /// How many are put, on a holder. One when not written.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        count: Option<CountSpec>,
    },
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
    /// Adds `by` (default 1, never 0) to the mover's counter, or to every player's with `to` set
    /// to `ALL` (`MOVER` is the default).
    Add {
        counter: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        by: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<String>,
    },
    /// Sets the mover's counter to 0, or every player's with `to` set to `ALL`.
    Reset {
        counter: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<String>,
    },
    /// When this way is played, the turn is not handed on: the same player moves again in the
    /// stage `name` of `turns.stages`, if they have a way it allows.
    Stage { name: String },
    /// Hands a decision to other players, and takes the place of `STAGE` (a way may not run both).
    /// When this way is played, each asked seat in turn answers one way in the stage `stage`
    /// (`players`, `mode`), and then the asker moves again in the stage `then` names, or the turn
    /// is handed on when `then` is not written.
    Ask {
        players: AskPlayers,
        stage: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mode: Option<AskMode>,
        /// With `TOGETHER` only: whether an answer is kept from the other players until every
        /// one has answered (the default), or takes effect as it arrives.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sealed: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        then: Option<AskThen>,
    },
    /// Rolls `dice` dice (one when not written). Each lands on one of `faces` with equal chance,
    /// and the pieces of that face go into the bag `at`, as `PLACE` puts them.
    Roll {
        at: BagRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        dice: Option<CountSpec>,
        /// The faces of a die: each a set of pieces, `{ "PIP": 3 }`. Written twice, a face comes
        /// up twice as often.
        faces: Vec<BTreeMap<String, u32>>,
    },
    /// Puts the pieces of the ordered bag `at` in a random order.
    Shuffle { at: BagRef },
}

/// Who an `ASK` asks: `OTHERS` (every player but the one asking, in turn order after them), `ALL`
/// (the same, and the one asking last), `MOVER` (the one asking, as the player of an answer asks
/// itself again), or the players named, in the order written.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum AskPlayers {
    Who(String),
    Named(Vec<String>),
}

/// The words `AskPlayers::Who` may be.
pub const ASK_WHO: [&str; 3] = ["OTHERS", "ALL", "MOVER"];

/// How an `ASK` asks: the seats answer one after another, or all at the same time, in any order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AskMode {
    InOrder,
    Together,
}

/// What follows the answers of an `ASK`: `{ "stage": NAME }`, the asker moving again in that stage.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AskThen {
    pub stage: String,
}

/// The `to` of a counter effect that takes every player.
pub const ALL_PLAYERS: &str = "ALL";
/// The `to` of a counter effect that takes the mover alone (the default).
pub const MOVER_ONLY: &str = "MOVER";

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
