//! "Siege", the proof of sides with their own rules: a defender of one lord and four guards, bound
//! to capture, against a raider with no piece on the board and twelve in reserve, bound to nothing.
//! The two gaps it needs, a `PLAYER` test in `must` and a different reserve for each side, are
//! tested by it; the unit tests of each are in the engine.

mod support;

use std::collections::BTreeMap;

use modern_chess_session::{Answer, Session, Status, View};
use support::Dice;

/// Games played, and the turns each may take, which a random game rarely needs.
const GAMES: u64 = 40;
const TURN_CAP: usize = 150;

const SIEGE: &str = include_str!("fixtures/siege.json");

fn session(seed: u64) -> Session {
    Session::new_with_seed(SIEGE, seed).expect("Siege reads")
}

/// Every answer the board offers now.
fn offered(view: &View) -> Vec<Answer> {
    let choice = &view.choice;
    let mut answers: Vec<Answer> = choice
        .places
        .iter()
        .map(|place| Answer::Place(place.name))
        .collect();
    answers.extend(choice.held.iter().cloned().map(Answer::Held));
    answers.extend(
        choice
            .actions
            .iter()
            .map(|action| Answer::Action(action.name.clone())),
    );
    answers
}

/// Plays one whole turn, picking each answer at random from what is offered.
fn play_a_turn(session: &mut Session, dice: &mut Dice) {
    let mover = session.view().turn;
    while matches!(session.view().status, Status::Playing) && session.view().turn == mover {
        let mut answers = offered(&session.view());
        assert!(!answers.is_empty(), "{mover} is offered nothing mid-turn");
        let answer = answers.swap_remove(dice.below(answers.len()));
        session
            .choose(answer)
            .expect("an offered answer is accepted");
    }
}

fn reavers(view: &View) -> usize {
    view.pieces
        .iter()
        .filter(|piece| piece.code == "REAVER")
        .count()
}

/// Whether some piece of `player` stands next to a piece of the other side, which it could take.
fn can_capture(view: &View, player: &str) -> bool {
    view.pieces
        .iter()
        .filter(|piece| piece.player == player)
        .any(|piece| {
            let (x, y) = piece.position.split_once(',').expect("a grid place");
            let (x, y): (i32, i32) = (x.parse().unwrap(), y.parse().unwrap());
            [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| {
                let at = format!("{},{}", x + dx, y + dy);
                view.pieces
                    .iter()
                    .any(|other| other.position == at && other.player != player)
            })
        })
}

fn hand<'a>(view: &'a View, player: &str) -> &'a BTreeMap<String, u32> {
    &view
        .bags
        .iter()
        .find(|bag| bag.name == "HAND" && bag.owner.as_deref() == Some(player))
        .expect("each side has a hand")
        .pieces
}

#[test]
fn each_side_starts_with_a_reserve_of_its_own() {
    let view = session(1).view();
    assert_eq!(hand(&view, "RAIDER").get("REAVER"), Some(&12));
    assert!(hand(&view, "DEFENDER").is_empty());
    let count = |code: &str| {
        view.pieces
            .iter()
            .filter(|piece| piece.code == code)
            .count()
    };
    assert_eq!((count("LORD"), count("GUARD"), count("REAVER")), (1, 4, 0));
}

/// Puts a raider next to the guard at 2,3: drops one on the edge, then steps it in.
fn raider_beside_a_guard(session: &mut Session, dice: &mut Dice) {
    // The raider drops on the edge, anywhere at all.
    session.choose(Answer::Held("REAVER".to_string())).unwrap();
    session.choose(Answer::Place("0,3".into())).unwrap();
    assert_eq!(session.view().turn, "DEFENDER");
    // The defender has nothing to take yet: any guard may step. Not the one at 2,3.
    session.choose(Answer::Place("3,2".into())).unwrap();
    session.choose(Answer::Place("3,1".into())).unwrap();
    assert_eq!(session.view().turn, "RAIDER");
    let _ = dice;
    // The raider steps beside the guard at 2,3.
    session.choose(Answer::Place("0,3".into())).unwrap();
    session.choose(Answer::Place("1,3".into())).unwrap();
    assert_eq!(session.view().turn, "DEFENDER");
}

#[test]
fn the_defender_is_bound_to_capture() {
    let mut session = session(1);
    raider_beside_a_guard(&mut session, &mut Dice(1));
    let view = session.view();
    assert!(can_capture(&view, "DEFENDER"));
    // Only the guard that can take is offered; the lord and the other guards cannot be picked.
    let pickable: Vec<String> = view
        .choice
        .places
        .iter()
        .map(|place| place.name.as_str().to_string())
        .collect();
    assert_eq!(pickable, vec!["2,3".to_string()], "{pickable:?}");
    session.choose(Answer::Place("2,3".into())).unwrap();
    let targets: Vec<String> = session
        .view()
        .choice
        .places
        .iter()
        .map(|place| place.name.as_str().to_string())
        .collect();
    assert_eq!(
        targets,
        vec!["1,3".to_string()],
        "only the capture is offered"
    );
}

#[test]
fn the_raider_is_not_bound_to_anything() {
    let mut session = session(1);
    raider_beside_a_guard(&mut session, &mut Dice(1));
    // The guard at 2,3 takes the raider at 1,3.
    session.choose(Answer::Place("2,3".into())).unwrap();
    session.choose(Answer::Place("1,3".into())).unwrap();
    // The raider drops one at 0,4, which touches nothing. The defender, with nothing to take,
    // steps the guard at 1,3 beside it.
    session.choose(Answer::Held("REAVER".to_string())).unwrap();
    session.choose(Answer::Place("0,4".into())).unwrap();
    assert!(!can_capture(&session.view(), "DEFENDER"));
    session.choose(Answer::Place("1,3".into())).unwrap();
    session.choose(Answer::Place("1,4".into())).unwrap();
    // The raider may take the guard, and may as well drop another: nothing binds it.
    let view = session.view();
    assert_eq!(view.turn, "RAIDER");
    assert!(can_capture(&view, "RAIDER"));
    assert!(
        view.choice.held.contains(&"REAVER".to_string()),
        "{:?}",
        view.choice
    );
    assert!(
        view.choice
            .places
            .iter()
            .any(|place| place.name.as_str() == "0,4"),
        "{:?}",
        view.choice
    );
}

#[test]
fn many_games_play_out_and_the_defender_never_declines_a_capture() {
    let mut raider_wins = 0;
    let mut defender_wins = 0;
    let mut declined_by_raider = 0;
    let mut captured_by_defender = 0;
    for seed in 0..GAMES {
        let mut session = session(seed);
        let mut dice = Dice(seed + 7);
        let mut turns = 0;
        while matches!(session.view().status, Status::Playing) && turns < TURN_CAP {
            turns += 1;
            let before = session.view();
            let mover = before.turn.clone();
            let could = can_capture(&before, &mover);
            let on_board = reavers(&before);
            play_a_turn(&mut session, &mut dice);
            let took = reavers(&session.view()) < on_board
                || session.view().pieces.len() < before.pieces.len();
            if mover == "DEFENDER" {
                assert_eq!(
                    could, took,
                    "seed {seed}, turn {turns}: the defender must take if it can"
                );
                captured_by_defender += usize::from(took);
            } else if could && !took {
                declined_by_raider += 1;
            }
        }
        match session.view().status {
            Status::Won { winner } if winner == "RAIDER" => raider_wins += 1,
            Status::Won { .. } => defender_wins += 1,
            _ => {}
        }
        // Whatever happened, no more than twelve raiders ever existed.
        let view = session.view();
        let in_hand = hand(&view, "RAIDER").get("REAVER").copied().unwrap_or(0) as usize;
        assert!(reavers(&view) + in_hand <= 12, "seed {seed}");
    }
    assert!(
        captured_by_defender > 0,
        "the defender took pieces in some game"
    );
    assert!(
        declined_by_raider > 0,
        "the raider was free to leave a capture, and sometimes did"
    );
    assert!(
        raider_wins > 0,
        "the raiders won {raider_wins}, the defender {defender_wins}"
    );
    assert!(
        defender_wins > 0,
        "the raiders won {raider_wins}, the defender {defender_wins}"
    );
}

/// Siege with one more end check, which draws when `PLAYER` holds for the player it is judged as.
fn siege_that_draws_when(viewpoint: &str, who: &str) -> Session {
    let mut spec: serde_json::Value = serde_json::from_str(SIEGE).unwrap();
    spec["end_checks"].as_array_mut().unwrap().insert(
        0,
        serde_json::json!({ "as": viewpoint, "when": { "test": "PLAYER", "is": [who] },
                            "then": { "outcome": "DRAW" } }),
    );
    Session::new_with_seed(&spec.to_string(), 1).unwrap()
}

#[test]
fn player_in_an_end_check_is_the_player_it_is_judged_as() {
    // After the raider's first turn the one who just moved is RAIDER and the next is DEFENDER.
    for (viewpoint, who, ends) in [
        ("MOVER", "RAIDER", true),
        ("MOVER", "DEFENDER", false),
        ("NEXT", "DEFENDER", true),
        ("NEXT", "RAIDER", false),
    ] {
        let mut session = siege_that_draws_when(viewpoint, who);
        session.choose(Answer::Held("REAVER".to_string())).unwrap();
        session.choose(Answer::Place("0,3".into())).unwrap();
        assert_eq!(
            matches!(session.view().status, Status::Draw),
            ends,
            "judged as {viewpoint}, PLAYER is [{who}]"
        );
    }
}
