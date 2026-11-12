//! Rules written by someone else may be wrong in any way. Loading them must fail with a message
//! or give a game that can be played without panicking; it must never take the process down.
mod support;

use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Mutex;

use modern_chess_session::Session;
use serde_json::{json, Value};
use support::{specs, step, Dice};

const STEPS: usize = 6;
const GAMES: [&str; 3] = ["go", "checkers", "ataxx"];

static LAST_PANIC: Mutex<String> = Mutex::new(String::new());

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
    outcome.map_err(|_| LAST_PANIC.lock().unwrap().clone())
}

#[test]
fn damaged_rules_never_panic() {
    std::panic::set_hook(Box::new(|info| {
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
        *LAST_PANIC.lock().unwrap() = format!("{place}: {message}");
    }));

    let mut by_place: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (name, text) in specs() {
        if !GAMES.contains(&name.as_str()) {
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
    let _ = std::panic::take_hook();

    let report: Vec<String> = by_place
        .iter()
        .map(|(place, cases)| format!("{place}\n    e.g. {} ({} cases)", cases[0], cases.len()))
        .collect();
    assert!(by_place.is_empty(), "\n{}", report.join("\n"));
}
