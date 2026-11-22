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
        "a piece's `drop_restrictions`; write `drop_conditions`, tests that must all hold for a \
         drop on a square",
    ))
}

/// Refuses a piece's `restrictions`.
pub(crate) fn restrictions<'de, D: Deserializer<'de>>(_: D) -> Result<OldField, D::Error> {
    Err(refused(
        "a piece's `restrictions`; write an IN_ZONE test on each move instead, as \
         { \"test\": \"IN_ZONE\", \"at\": { \"place\": \"TARGET\" }, \"zone\": \"PALACE\" }",
    ))
}

/// Refuses a side effect's `condition`.
pub(crate) fn side_effect_condition<'de, D: Deserializer<'de>>(_: D) -> Result<OldField, D::Error> {
    Err(refused(
        "a side effect's `condition`, which was never judged; put the test on the move or the \
         action, in its `conditions`",
    ))
}

/// Reads a list of tests, refusing a condition written as before tests.
pub(crate) fn tests<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<TestSpec>, D::Error> {
    Vec::<Value>::deserialize(deserializer)?
        .into_iter()
        .map(test)
        .collect()
}

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
