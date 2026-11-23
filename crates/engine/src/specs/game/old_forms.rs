//! Forms the spec format no longer has. Reading one is refused, with a message naming what
//! replaced it.

use serde::de::Error;
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::game::GameSpecError;
use super::test_block::TestSpec;

/// What a refusal of an old form starts with, so that `old_form_refusal` can find it.
const OLD_FORM_PREFIX: &str = "old form: ";

/// A field the spec format no longer has. It is never read: writing it is refused.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OldField;

fn refused<E: Error>(why: impl std::fmt::Display) -> E {
    E::custom(format!("{OLD_FORM_PREFIX}{why}"))
}

/// Refuses the top-level `conditions` list.
pub(crate) fn top_level_conditions<'de, D: Deserializer<'de>>(_: D) -> Result<OldField, D::Error> {
    Err(refused(
        "the top-level `conditions` list; write each zone under `zones`, as \
         { \"name\": \"PALACE\", \"places\": { \"WHITE\": [...] } }",
    ))
}

/// Refuses a piece's `drop_restrictions`.
pub(crate) fn drop_restrictions<'de, D: Deserializer<'de>>(_: D) -> Result<OldField, D::Error> {
    Err(refused(
        "a piece's `drop_restrictions`; write tests in the drop action, an action that chooses \
         a HAND piece and a PLACE, then tests the TARGET square",
    ))
}

/// Refuses a piece's `restrictions`.
pub(crate) fn restrictions<'de, D: Deserializer<'de>>(_: D) -> Result<OldField, D::Error> {
    Err(refused(
        "a piece's `restrictions`; write an IN_ZONE test on each move instead, as \
         { \"test\": \"IN_ZONE\", \"at\": { \"place\": \"TARGET\" }, \"zone\": \"PALACE\" }",
    ))
}

/// Defines the reader of a field that is refused whenever it is written, whatever it holds.
macro_rules! refuse_field {
    ($name:ident, $doc:literal, $why:literal) => {
        #[doc = $doc]
        pub(crate) fn $name<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<OldField, D::Error> {
            serde::de::IgnoredAny::deserialize(deserializer)?;
            Err(refused($why))
        }
    };
}

refuse_field!(
    move_actions,
    "Refuses a move's `actions`, which were keyed by what stands on the target.",
    "a move's `actions`; write the move's `do`, steps that test what stands on TARGET and then \
     do the effects, such as [ { \"test\": ... }, { \"effect\": \"MOVE\", \"from\": \
     { \"place\": \"START\" }, \"to\": { \"place\": \"TARGET\" } } ]"
);

refuse_field!(
    move_conditions,
    "Refuses a move's `conditions`.",
    "a move's `conditions`; write them as test steps at the start of the move's `do`"
);

refuse_field!(
    move_modifiers,
    "Refuses a move's `modifiers`.",
    "a move's `modifiers`; write a promotion in the move's `do`, as an `if` over the zone with \
     a CODE choice and a SET_CODE effect in its `then`"
);

refuse_field!(
    move_side_effects,
    "Refuses a move's `side_effects`.",
    "a move's `side_effects`; write each as an effect step in the move's `do` (REMOVE, MOVE, \
     PLACE, SET_OWNER, SET_CODE, SET_FLAG, FOR_EACH)"
);

refuse_field!(
    allow_pass,
    "Refuses the top-level `allow_pass`.",
    "`allow_pass`; write an action with an empty `do`, as { \"name\": \"PASS\", \"do\": [] }"
);

refuse_field!(
    unlimited_supply,
    "Refuses a piece's `unlimited_supply`.",
    "a piece's `unlimited_supply`; write an action that does `PLACE`, as { \"name\": \"STONE\", \
     \"do\": [ { \"choose\": \"PLACE\", \"in\": { \"region\": \"BOARD\" } }, ..., \
     { \"effect\": \"PLACE\", \"at\": { \"place\": \"TARGET\" }, \"code\": \"STONE\" } ] }"
);

refuse_field!(
    drop_conditions,
    "Refuses a piece's `drop_conditions`.",
    "a piece's `drop_conditions`; write the tests in the drop action, after its HAND and PLACE \
     choices, judged with TARGET the square and START the piece in the hand"
);

fn test<E: Error>(value: Value) -> Result<TestSpec, E> {
    if let (None, Some(name)) = (value.get("test"), value.get("condition")) {
        return Err(refused(format!(
            "the condition {name}; write it as a test block, such as \
             {{ \"test\": \"COUNT\", \"in\": {{ \"region\": \"AT\", \"at\": {{ \"place\": \"TARGET\" }} }} }}"
        )));
    }
    TestSpec::deserialize(value).map_err(E::custom)
}

/// Reads a repeat's `until`, refusing a condition named as before tests. `null` is no `until`.
pub(crate) fn until<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<TestSpec>, D::Error> {
    match Value::deserialize(deserializer)? {
        Value::String(name) => Err(refused(format!(
            "a repeat's `until` written as the name `{name}`; write a test, judged on the next \
             square (TARGET): leave it out to stop once that square is occupied, or write \
             {{ \"test\": \"ANY\", \"of\": [] }} never to stop early"
        ))),
        Value::Null => Ok(None),
        value => test(value).map(Some),
    }
}

/// The refusal of an old form, when `error` (from reading a spec) carries one, with where in
/// the text the old form was written.
pub(crate) fn old_form_refusal(error: &serde_json::Error) -> Option<GameSpecError> {
    let message = error.to_string();
    let start = message.find(OLD_FORM_PREFIX)?;
    let rest = &message[start + OLD_FORM_PREFIX.len()..];
    let why = match rest.rsplit_once(" at line ") {
        Some((why, place)) => format!("{why} (at line {place})"),
        None => rest.to_string(),
    };
    Some(GameSpecError::OldForm(why))
}
