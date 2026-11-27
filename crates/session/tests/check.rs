//! `check_spec` tells a person writing a spec what is wrong with it and where, in words, not in
//! the engine's debug text. The builder shows this next to the editor.

use modern_chess_session::check_spec;

const CHESS: &str = include_str!("../../engine/specs/chess.json");
const MARCH: &str = include_str!("../../../apps/docs/examples/march.json");

fn pretty(text: &str) -> serde_json::Value {
    serde_json::from_str(text).unwrap()
}

#[test]
fn a_good_spec_is_summarised() {
    let summary = check_spec(CHESS).expect("chess is a good spec");
    assert_eq!(summary.name, "CHESS");
    assert_eq!(summary.players, vec!["WHITE", "BLACK"]);
}

#[test]
fn text_that_is_not_json_says_where_it_stops() {
    let problem = check_spec("{\n  \"name\": \n").expect_err("not JSON");
    assert!(problem.message.contains("EOF"), "{}", problem.message);
    assert_eq!(problem.line, Some(3));
    assert!(
        !problem.message.contains(" at line "),
        "the place is not repeated in the words"
    );
}

#[test]
fn a_key_the_format_does_not_have_is_found_by_line_and_path() {
    let mut spec = pretty(MARCH);
    spec["board"]["wide"] = serde_json::json!(3);
    let text = serde_json::to_string_pretty(&spec).unwrap();

    let problem = check_spec(&text).expect_err("an unknown key");

    assert!(
        problem.message.contains("unknown field `wide`"),
        "{}",
        problem.message
    );
    assert_eq!(problem.path.as_deref(), Some("board.wide"));
    let line = problem.line.expect("a line");
    assert!(
        text.lines().nth(line - 1).unwrap().contains("wide"),
        "line {line}"
    );
}

#[test]
fn a_missing_format_version_is_said_in_words() {
    let mut spec = pretty(MARCH);
    spec.as_object_mut().unwrap().remove("format_version");

    let problem = check_spec(&spec.to_string()).expect_err("no version");

    assert!(
        problem.message.contains("format_version"),
        "{}",
        problem.message
    );
    assert!(
        !problem.message.contains("MissingFormatVersion"),
        "{}",
        problem.message
    );
    assert_eq!(problem.line, None);
}

#[test]
fn a_rule_that_contradicts_the_rest_is_said_in_words_with_no_line() {
    let text = MARCH.replacen("\"MARCHER\"", "\"MARCHERS\"", 1);

    let problem = check_spec(&text).expect_err("a piece that is not defined");

    assert!(problem.message.contains("MARCHERS"), "{}", problem.message);
    assert!(
        !problem.message.contains("UnknownPiece"),
        "{}",
        problem.message
    );
    assert_eq!(problem.line, None);
}
