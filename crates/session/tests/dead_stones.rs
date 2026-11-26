//! Go's dead-stone agreement, as the catalogue's Go has it: the second pass in a row opens a
//! marking for the player who passed, who marks the stones that are dead, a whole group at a time,
//! and asks the other, who agrees (the marked stones are taken off and the game is scored) or
//! disputes (the stones are alive again and play goes on with the next player). A group marked
//! dead is revived by marking it again. The ko rule holds through all of it: it is the
//! `REPEATS` of the stone's rule, written in a game with stages. It proves `ASK`, `STAGE`, the
//! actions of a stage and the asked seat being the one to move, over the session.

use modern_chess_session::{Answer, Session, Status, View};

const GO: &str = include_str!("../../engine/specs/go.json");
const GO_19: &str = include_str!("../../engine/specs/go_19.json");

fn session() -> Session {
    Session::new(GO).expect("Go reads")
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
fn one_pass_does_not_open_the_marking() {
    let mut session = session();
    place(&mut session, "0,0");
    act(&mut session, "PASS");
    let view = session.view();
    assert_eq!(view.turn, "BLACK");
    assert!(actions(&view).contains(&"PASS".to_string()));
    assert!(!actions(&view).contains(&"DONE".to_string()));
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
fn marking_a_stone_marks_its_whole_group_and_marking_it_again_revives_the_group() {
    let mut session = session();
    // Black: a group of three along the edge and a lone stone; White: two stones.
    for (black, white) in [("0,0", "8,8"), ("0,1", "8,7"), ("0,2", "7,8")] {
        place(&mut session, black);
        place(&mut session, white);
    }
    place(&mut session, "5,5");
    act(&mut session, "PASS");
    act(&mut session, "PASS");
    // White passed, then Black: Black marks the group of its own three stones, by one of them.
    place(&mut session, "0,1");
    let view = session.view();
    assert_eq!(count(&view, "DEAD"), 3, "the whole group is marked");
    assert_eq!(count(&view, "STONE"), 4, "the other stones are not");
    assert_eq!(view.turn, "BLACK", "marking goes on");
    // Marking a dead stone gives its group back.
    place(&mut session, "0,0");
    let view = session.view();
    assert_eq!(count(&view, "DEAD"), 0);
    assert_eq!(count(&view, "STONE"), 7);
}

#[test]
fn marking_the_stones_of_one_player_leaves_a_touching_group_of_the_other_alone() {
    let mut session = session();
    place(&mut session, "3,3");
    place(&mut session, "3,4");
    act(&mut session, "PASS");
    act(&mut session, "PASS");
    place(&mut session, "3,3");
    let view = session.view();
    assert_eq!(count(&view, "DEAD"), 1, "only the stone's own group");
    assert_eq!(count(&view, "STONE"), 1);
}

#[test]
fn a_dispute_brings_the_stones_back_and_play_goes_on_with_the_next_player() {
    let mut session = two_passes();
    place(&mut session, "0,0");
    act(&mut session, "DONE");
    act(&mut session, "DISPUTE");
    let view = session.view();
    assert!(matches!(view.status, Status::Playing));
    assert_eq!(count(&view, "DEAD"), 0, "the stones are alive again");
    assert_eq!(view.turn, "BLACK", "play resumes with the next player");
    assert!(actions(&view).contains(&"PASS".to_string()));
    assert!(!actions(&view).contains(&"DONE".to_string()));
    assert!(
        !view.choice.places.is_empty(),
        "stones may be placed again: {:?}",
        view.choice
    );
}

#[test]
fn after_a_dispute_the_players_can_pass_again_and_mark_again() {
    let mut session = two_passes();
    act(&mut session, "DONE");
    act(&mut session, "DISPUTE");
    place(&mut session, "2,2");
    place(&mut session, "6,6");
    act(&mut session, "PASS");
    act(&mut session, "PASS");
    // BLACK passed first, WHITE second: WHITE marks again. This time BLACK's stone at 0,0 and
    // 2,2 are dead.
    place(&mut session, "0,0");
    place(&mut session, "2,2");
    act(&mut session, "DONE");
    assert_eq!(session.view().turn, "BLACK");
    act(&mut session, "AGREE");
    let view = session.view();
    assert_eq!(count(&view, "STONE") + count(&view, "DEAD"), 2);
    match view.status {
        Status::Won { winner } => assert_eq!(winner, "WHITE"),
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
    // Back to play: the answers are not offered any more.
    assert!(session.choose(Answer::Action("AGREE".into())).is_err());
}

/// A ko in the middle of the board, 9x9 coordinates as x,y:
///
/// ```text
///      2 3 4 5
///  y=2 . B W .
///  y=3 B . B W
///  y=4 . B W .
/// ```
///
/// Black has just played 4,3 and White is to move: White takes it by playing 3,3.
fn ko() -> Session {
    let mut session = session();
    for at in [
        "3,2", "4,2", "2,3", "5,3", "3,4", "4,4", // the shape, Black and White in turn
        "4,3", // Black fills the ko point
        "3,3", // White takes it
    ] {
        place(&mut session, at);
    }
    session
}

fn can_place(session: &Session, at: &str) -> bool {
    session
        .view()
        .choice
        .places
        .iter()
        .any(|place| place.name.as_str() == at)
}

#[test]
fn the_ko_rule_holds_in_a_game_with_stages() {
    let session = ko();
    assert_eq!(count(&session.view(), "STONE"), 7, "White took one stone");
    assert_eq!(session.view().turn, "BLACK");
    assert!(
        !can_place(&session, "4,3"),
        "Black may not take back at once: the position would be the one before White's move"
    );
    assert!(can_place(&session, "0,0"), "other points are open");
}

#[test]
fn the_ko_still_holds_after_the_players_passed_and_one_disputed() {
    let mut session = ko();
    act(&mut session, "PASS");
    act(&mut session, "PASS");
    // White passed second and marks nothing: it asks, and Black disputes.
    act(&mut session, "DONE");
    act(&mut session, "DISPUTE");
    assert_eq!(session.view().turn, "BLACK", "play resumes");
    assert!(
        !can_place(&session, "4,3"),
        "a position from before the passes still cannot return"
    );
}

#[test]
fn the_ko_is_a_rule_of_every_size_of_the_board() {
    let session = Session::new(GO_19).expect("Go 19 reads");
    assert!(can_place(&session, "3,3"));
}

#[test]
fn the_catalogue_go_ends_only_by_agreement() {
    // Passing and passing is not the end: the stage is.
    let mut session = session();
    act(&mut session, "PASS");
    act(&mut session, "PASS");
    assert!(matches!(session.view().status, Status::Playing));
    act(&mut session, "DONE");
    act(&mut session, "AGREE");
    assert!(!matches!(session.view().status, Status::Playing));
}
