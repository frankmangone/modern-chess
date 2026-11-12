//! Rules written by someone else may be wrong in any way. Loading them must fail with a message
//! or give a game that can be played without panicking; it must never take the process down.
mod support;

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Once;

use modern_chess_session::Session;
use serde_json::{json, Value};
use support::{specs, step, Dice};

const STEPS: usize = 6;
const GAMES: [&str; 3] = ["go", "checkers", "ataxx"];

thread_local! {
    /// Where and why this thread last panicked while trying a damaged spec.
    static LAST_PANIC: RefCell<String> = const { RefCell::new(String::new()) };
    /// True while a damaged spec is being tried; any other panic (a failed assertion) is
    /// left to the default hook so its message still reaches the test report.
    static TRYING: Cell<bool> = const { Cell::new(false) };
}

static REMEMBER_PANICS: Once = Once::new();

/// Installed once for the whole test binary, so tests running side by side do not undo it.
fn remember_panics() {
    REMEMBER_PANICS.call_once(|| {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if !TRYING.with(Cell::get) {
                default_hook(info);
                return;
            }
            let place = info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_default();
            let message = info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_default();
            LAST_PANIC.with(|last| *last.borrow_mut() = format!("{place}: {message}"));
        }));
    });
}

/// Every place in the document a value can sit, as a list of keys and indexes.
fn paths(value: &Value, here: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    out.push(here.clone());
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                here.push(key.clone());
                paths(child, here, out);
                here.pop();
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                here.push(index.to_string());
                paths(child, here, out);
                here.pop();
            }
        }
        _ => {}
    }
}

fn at<'a>(root: &'a mut Value, path: &[String]) -> &'a mut Value {
    path.iter().fold(root, |node, key| match node {
        Value::Array(items) => &mut items[key.parse::<usize>().unwrap()],
        other => &mut other[key.as_str()],
    })
}

/// The ways a value can be wrong. `None` removes it.
fn damage() -> Vec<(&'static str, Option<Value>)> {
    vec![
        ("removed", None),
        ("null", Some(Value::Null)),
        ("zero", Some(json!(0))),
        ("negative", Some(json!(-1))),
        ("300", Some(json!(300))),
        ("huge", Some(json!(4_000_000_000u64))),
        ("empty string", Some(json!(""))),
        ("unknown name", Some(json!("NOPE"))),
        ("empty list", Some(json!([]))),
        ("empty object", Some(json!({}))),
    ]
}

fn remove(root: &mut Value, path: &[String]) {
    let (last, parent) = path.split_last().unwrap();
    match at(root, parent) {
        Value::Array(items) => {
            items.remove(last.parse::<usize>().unwrap());
        }
        Value::Object(map) => {
            map.remove(last.as_str());
        }
        _ => {}
    }
}

fn try_spec(text: &str) -> Result<(), String> {
    TRYING.with(|trying| trying.set(true));
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let Ok(mut session) = Session::new(text) else {
            return;
        };
        let mut dice = Dice(7);
        for _ in 0..STEPS {
            step(&mut session, &mut dice);
            let _ = serde_json::to_string(&session.view());
        }
    }));
    TRYING.with(|trying| trying.set(false));
    outcome.map_err(|_| LAST_PANIC.with(|last| last.borrow().clone()))
}

/// Damages every value of every spec `wanted` accepts, one at a time, and reports each place
/// in the code that panicked with one example.
fn assert_no_panics(wanted: impl Fn(&str) -> bool) {
    remember_panics();

    let mut by_place: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, text) in specs() {
        if !wanted(&name) {
            continue;
        }
        let original: Value = serde_json::from_str(&text).unwrap();
        let mut all = Vec::new();
        paths(&original, &mut Vec::new(), &mut all);
        for path in all.iter().filter(|p| !p.is_empty()) {
            for (label, replacement) in damage() {
                let mut copy = original.clone();
                match replacement {
                    Some(value) => *at(&mut copy, path) = value,
                    None => remove(&mut copy, path),
                }
                if let Err(panic) = try_spec(&copy.to_string()) {
                    by_place
                        .entry(panic)
                        .or_default()
                        .push(format!("{name} {} -> {label}", path.join("/")));
                }
            }
        }
    }
    let report: Vec<String> = by_place
        .iter()
        .map(|(place, cases)| format!("{place}\n    e.g. {} ({} cases)", cases[0], cases.len()))
        .collect();
    assert!(by_place.is_empty(), "\n{}", report.join("\n"));
}

/// The quick check, run with every test: the games that have found bugs so far.
#[test]
fn damaged_rules_never_panic() {
    assert_no_panics(|name| GAMES.contains(&name));
}

/// Every shipped game. Takes a few minutes, so it runs on request: `make test-damage`.
#[test]
#[ignore = "slow; run with `make test-damage`"]
fn damaged_rules_never_panic_in_any_game() {
    assert_no_panics(|_| true);
}
