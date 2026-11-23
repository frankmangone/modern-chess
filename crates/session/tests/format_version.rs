//! A session refuses a spec that does not state the format version it reads. The wasm build and
//! the game server open specs through the same parser. (The session shows an error as its debug
//! text, so it names the variant.)

use modern_chess_session::Session;

const CHESS: &str = include_str!("../../engine/specs/chess.json");

#[test]
fn a_session_opens_a_spec_that_states_version_one() {
    assert!(Session::new(CHESS).is_ok());
}

#[test]
fn a_session_refuses_a_spec_with_no_version_or_another_one() {
    let mut spec: serde_json::Value = serde_json::from_str(CHESS).unwrap();
    spec["format_version"] = serde_json::json!(2);
    let error = Session::new(&spec.to_string())
        .err()
        .expect("version 2 is refused");
    assert!(error.contains("FormatVersion"), "{error}");

    spec.as_object_mut().unwrap().remove("format_version");
    let error = Session::new(&spec.to_string())
        .err()
        .expect("no version is refused");
    assert!(error.contains("FormatVersion"), "{error}");
}
