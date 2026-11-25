//! Go's dead-stone agreement, written as a variant of the catalogue's Go: after two passes the
//! player who passed second marks the stones that are dead, and asks the other, who agrees (the
//! marked stones are taken off and the game is scored) or disputes (the stones are alive again
//! and the first player marks anew). It proves `ASK` one after another, `then`, actions that
//! belong to a stage, and the asked seat being the one to move, over the session. (It has no ko
//! rule: `REPEATS` is refused in a game with stages, an existing limit that `ASK` shares.)

use modern_chess_session::{Answer, Session, Status, View};
use serde_json::{json, Value};

const GO: &str = include_str!("../../engine/specs/go.json");

/// Every stone of `code` on the board.
fn each_of(code: &str) -> Value {
    json!({ "region": "BOARD", "where": { "codes": [code] } })
}

/// Turns every stone that is `from` into `to`, wherever it is on the board.
fn turn_all(from: &str, to: &str) -> Value {
    json!({ "effect": "FOR_EACH", "in": each_of(from),
            "do": [{ "effect": "SET_CODE", "at": { "place": "EACH" }, "code": to }] })
}

fn nothing_agreed() -> Value {
    json!({ "test": "COUNTER", "name": "AGREED", "max": 0 })
}

fn variant() -> String {
    let mut spec: Value = serde_json::from_str(GO).unwrap();
    spec["board"]["dimensions"] = json!([5, 5]);
    spec["counters"] = json!(["AGREED"]);
    spec["pieces"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "code": "DEAD", "name": "dead stone", "moves": [] }));
    spec["turns"]["stages"] = json!([{ "name": "MARKING" }, { "name": "REPLY" }]);
    // A game with stages cannot judge `REPEATS` (a stage works the ways out again mid-turn), so
    // this Go has no ko rule: the last test of the stone's rule goes.
    let rules = spec["every_action"][0]["do"].as_array_mut().unwrap();
    assert_eq!(rules.pop().unwrap()["test"], "NOT");
    // The second pass in a row opens the marking.
    spec["actions"][1] = json!({ "name": "PASS", "do": [{
        "if": { "test": "PASSES", "in_a_row": 1 },
        "then": [{ "effect": "STAGE", "name": "MARKING" }]
    }] });
    let actions = spec["actions"].as_array_mut().unwrap();
    actions.push(json!({ "name": "MARK", "stage": "MARKING", "do": [
        nothing_agreed(),
        { "choose": "PIECE", "where": { "codes": ["STONE"] } },
        { "effect": "SET_CODE", "at": { "place": "START" }, "code": "DEAD" },
        { "effect": "STAGE", "name": "MARKING" }
    ] }));
    actions.push(json!({ "name": "DONE", "stage": "MARKING", "do": [
        nothing_agreed(),
        { "effect": "ASK", "players": "OTHERS", "stage": "REPLY", "then": { "stage": "MARKING" } }
    ] }));
    actions.push(json!({ "name": "RESUME", "stage": "MARKING", "do": [
        nothing_agreed(), turn_all("DEAD", "STONE")
    ] }));
    actions.push(json!({ "name": "AGREE", "stage": "REPLY", "do": [
        { "effect": "ADD", "counter": "AGREED", "to": "ALL" },
        { "effect": "FOR_EACH", "in": each_of("DEAD"),
          "do": [{ "effect": "REMOVE", "at": { "place": "EACH" } }] }
    ] }));
    actions.push(json!({ "name": "DISPUTE", "stage": "REPLY", "do": [turn_all("DEAD", "STONE")] }));
    spec["end_checks"] = json!([{
        "when": { "test": "COUNTER", "name": "AGREED", "min": 1 }, "then": { "outcome": "HIGHEST" }
    }]);
    spec.to_string()
}

fn session() -> Session {
    Session::new(&variant()).expect("the variant reads")
}

fn place(session: &mut Session, at: &str) {
    session.choose(Answer::Place(at.into())).unwrap();
}

fn act(session: &mut Session, name: &str) {
    session.choose(Answer::Action(name.to_string())).unwrap();
}

fn actions(view: &View) -> Vec<String> {
    let mut names: Vec<String> = view.choice.actions.iter().map(|a| a.name.clone()).collect();
    names.sort();
    names
}

fn count(view: &View, code: &str) -> usize {
    view.pieces
        .iter()
        .filter(|piece| piece.code == code)
        .count()
}

/// Black plays 0,0 and White 4,4, then both pass: White is to mark.
fn two_passes() -> Session {
    let mut session = session();
    place(&mut session, "0,0");
    place(&mut session, "4,4");
    act(&mut session, "PASS");
    act(&mut session, "PASS");
    session
}

#[test]
fn the_second_pass_opens_a_marking_for_the_one_who_passed() {
    let session = two_passes();
    let view = session.view();
    assert!(matches!(view.status, Status::Playing));
    assert_eq!(view.turn, "WHITE");
    assert_eq!(actions(&view), ["DONE", "MARK", "RESUME"]);
    assert_eq!(
        view.choice.places.len(),
        2,
        "any stone may be marked: {:?}",
        view.choice
    );
}

#[test]
fn the_asked_player_agrees_and_the_marked_stones_are_taken_and_the_game_scored() {
    let mut session = two_passes();
    place(&mut session, "0,0");
    assert_eq!(session.view().turn, "WHITE", "marking goes on");
    assert_eq!(count(&session.view(), "DEAD"), 1);
    act(&mut session, "DONE");
    // The asked player is to move, with only the answers.
    let view = session.view();
    assert_eq!(view.turn, "BLACK");
    assert_eq!(actions(&view), ["AGREE", "DISPUTE"]);
    assert!(view.choice.places.is_empty());
    act(&mut session, "AGREE");
    let view = session.view();
    assert_eq!(
        count(&view, "DEAD") + count(&view, "STONE"),
        1,
        "only 4,4 is left"
    );
    match view.status {
        Status::Won { winner } => assert_eq!(winner, "WHITE"),
        other => panic!("the game is scored: {other:?}"),
    }
}

#[test]
fn a_dispute_brings_the_stones_back_and_the_first_player_marks_again() {
    let mut session = two_passes();
    place(&mut session, "0,0");
    act(&mut session, "DONE");
    act(&mut session, "DISPUTE");
    let view = session.view();
    assert!(matches!(view.status, Status::Playing));
    assert_eq!(count(&view, "DEAD"), 0, "the stones are alive again");
    assert_eq!(view.turn, "WHITE", "the asker moves again");
    assert_eq!(actions(&view), ["DONE", "MARK", "RESUME"]);
    // This time White marks its own stone at 4,4 (giving the game up), and Black agrees.
    place(&mut session, "4,4");
    act(&mut session, "DONE");
    assert_eq!(session.view().turn, "BLACK");
    act(&mut session, "AGREE");
    let view = session.view();
    assert_eq!(
        count(&view, "STONE") + count(&view, "DEAD"),
        1,
        "0,0 is left"
    );
    match view.status {
        Status::Won { winner } => assert_eq!(winner, "BLACK"),
        other => panic!("the game is scored: {other:?}"),
    }
}

#[test]
fn the_asker_may_give_up_marking_and_play_goes_on() {
    let mut session = two_passes();
    place(&mut session, "0,0");
    act(&mut session, "RESUME");
    let view = session.view();
    assert!(matches!(view.status, Status::Playing));
    assert_eq!(count(&view, "DEAD"), 0);
    assert_eq!(view.turn, "BLACK", "the turn is handed on");
    assert!(view.choice.actions.iter().any(|a| a.name == "PASS"));
}

#[test]
fn the_asked_player_cannot_play_a_stone_or_mark_and_the_asker_cannot_answer_for_them() {
    let mut session = two_passes();
    act(&mut session, "DONE");
    assert!(session.choose(Answer::Place("2,2".into())).is_err());
    assert!(session.choose(Answer::Action("MARK".into())).is_err());
    assert!(session.choose(Answer::Action("PASS".into())).is_err());
    act(&mut session, "DISPUTE");
    // Back with the asker, whose answers are the marking's alone.
    assert!(session.choose(Answer::Action("AGREE".into())).is_err());
}

#[test]
fn a_game_with_no_agreement_is_played_as_the_catalogue_plays_it() {
    // The catalogue's Go is untouched by the new effect: two passes end it.
    let mut session = Session::new(GO).unwrap();
    session.choose(Answer::Action("PASS".into())).unwrap();
    session.choose(Answer::Action("PASS".into())).unwrap();
    assert!(!matches!(session.view().status, Status::Playing));
}
