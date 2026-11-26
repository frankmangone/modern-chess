//! Backgammon through the session: the view the players are shown (24 points and two trays, the
//! dice and the bar in each player's own bags), a game played to its end with the match saved
//! and resumed after every answer (the stage in the middle of a turn included), and what a client
//! is told of the dice.

use modern_chess_session::{Answer, Session, Status, View};

const GAME: &str = include_str!("../../engine/specs/backgammon.json");

fn count_at(view: &View, point: &str, code: &str) -> u32 {
    // A point lists a kind once for each owner it was kept under (the first checkers are nobody's).
    view.pieces
        .iter()
        .filter(|piece| piece.position == point && piece.code == code)
        .map(|piece| piece.count.unwrap_or(1))
        .sum()
}

fn bag<'a>(view: &'a View, name: &str, owner: &str) -> &'a std::collections::BTreeMap<String, u32> {
    &view
        .bags
        .iter()
        .find(|bag| bag.name == name && bag.owner.as_deref() == Some(owner))
        .unwrap_or_else(|| panic!("{owner} has a {name}"))
        .pieces
}

/// The first thing the mover is offered: a point, or else an action.
fn first_answer(view: &View) -> Answer {
    match view.choice.places.first() {
        Some(place) => Answer::Place(place.name.clone()),
        None => Answer::Action(view.choice.actions[0].name.clone()),
    }
}

#[test]
fn the_view_is_a_map_of_the_points_and_the_trays_with_white_to_roll() {
    let session = Session::new_with_seed(GAME, 7).unwrap();
    let view = session.view();
    assert_eq!(view.nodes.len(), 26, "twenty-four points and a tray each");
    assert_eq!(view.turn, "WHITE");
    assert_eq!(count_at(&view, "24", "WHITE_CHECKER"), 2);
    assert_eq!(count_at(&view, "6", "WHITE_CHECKER"), 5);
    assert_eq!(count_at(&view, "19", "BLACK_CHECKER"), 5);
    let actions: Vec<_> = view.choice.actions.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(actions, ["ROLL"]);
    assert!(view.choice.places.is_empty(), "the roll comes first");
    assert!(bag(&view, "DICE", "WHITE").is_empty());
    let bar = view.bags.iter().find(|bag| bag.name == "Black bar").expect("a bar for black");
    assert!(bar.pieces.is_empty() && bar.owner.is_none());
}

#[test]
fn a_roll_shows_the_dice_in_the_movers_bag_and_never_the_seed() {
    let mut session = Session::new_with_seed(GAME, 7).unwrap();
    session.choose(Answer::Action("ROLL".to_string())).unwrap();
    let view = session.view();
    assert_eq!(view.turn, "WHITE", "the same player plays the dice");
    let dice: u32 = bag(&view, "DICE", "WHITE").values().sum();
    assert!(dice == 2 || dice == 4, "two dice, or four on a double: {dice}");
    assert!(!view.choice.places.is_empty(), "points to play to");
    let shown = serde_json::to_string(&view).unwrap();
    assert!(!shown.contains("\"seed\"") && !shown.contains("draws"), "{shown}");
}

#[test]
fn a_game_is_played_to_its_end_saved_and_resumed_after_every_answer() {
    for seed in [1_u64, 2, 3] {
        let mut straight = Session::new_with_seed(GAME, seed).unwrap();
        let mut stored = Session::new_with_seed(GAME, seed).unwrap();
        let mut answers = 0;
        while matches!(straight.view().status, Status::Playing) {
            let answer = first_answer(&straight.view());
            straight.choose(answer.clone()).unwrap();
            stored.choose(answer).unwrap();
            let state = stored.state();
            stored = Session::resume(GAME, &state, answers + 1).unwrap();
            assert_eq!(straight.view(), stored.view(), "seed {seed}, answer {answers}");
            // A point lists each kind of checker once: they are all nobody's, so a checker that
            // came off the bar does not make a second row beside the ones that started there.
            let mut kinds: Vec<_> = straight.view().pieces.iter().map(|p| (p.position.clone(), p.code.clone())).collect();
            let all = kinds.len();
            kinds.sort();
            kinds.dedup();
            assert_eq!(kinds.len(), all, "seed {seed}, answer {answers}: a kind is listed twice");
            answers += 1;
            assert!(answers < 20_000, "seed {seed} never ends");
        }
        match straight.view().status {
            Status::Won { winner } => {
                let tray = if winner == "WHITE" { "OFFW" } else { "OFFB" };
                let code = format!("{winner}_CHECKER");
                assert_eq!(count_at(&straight.view(), tray, &code), 15, "the winner has all fifteen off");
            }
            other => panic!("a game of dice ends with a winner: {other:?}"),
        }
    }
}

#[test]
fn another_seed_plays_another_game() {
    let play = |seed| {
        let mut session = Session::new_with_seed(GAME, seed).unwrap();
        for _ in 0..6 {
            let answer = first_answer(&session.view());
            session.choose(answer).unwrap();
        }
        serde_json::to_string(&session.view()).unwrap()
    };
    assert_eq!(play(5), play(5));
    assert!((0..20).any(|seed| play(seed) != play(5)), "seeds differ");
}
