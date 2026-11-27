//! Rock-paper-scissors, the proof of asking everyone at once (`ASK` with `TOGETHER`). Each round
//! both players throw, in either order, and neither sees the other's throw until both are in; a
//! round that is drawn is played again. The same game with `sealed` off is the open mode, where a
//! throw takes effect as it comes and the second player sees the first's.

mod support;

use modern_chess_session::{Answer, Session, Status, View, Viewer};
use serde_json::{json, Value};
use support::Dice;

const RPS: &str = include_str!("../../engine/specs/rock_paper_scissors.json");

fn sealed() -> Session {
    Session::new(RPS).expect("rock-paper-scissors reads")
}

/// The same game with the throws in the open: nothing is kept, and everyone sees a pick.
fn open() -> Session {
    let mut spec: Value = serde_json::from_str(RPS).unwrap();
    spec["actions"][0]["do"][2]["sealed"] = json!(false);
    spec["bags"][0].as_object_mut().unwrap().remove("visible");
    Session::new(&spec.to_string()).expect("the open game reads")
}

fn seat(name: &str) -> Viewer {
    Viewer::Player(name.to_string())
}

fn throw(session: &mut Session, who: &str, hand: &str) {
    session
        .choose_as(Some(who), Answer::Code(hand.to_string()))
        .unwrap_or_else(|error| panic!("{who} cannot throw {hand}: {error}"));
}

fn round(session: &mut Session) {
    session.choose(Answer::Action("PLAY".to_string())).unwrap();
}

/// The throw `of` has made, as `viewer` sees it: its pick bag.
fn pick(view: &View, of: &str) -> (Vec<String>, bool, Option<u32>) {
    let bag = view
        .bags
        .iter()
        .find(|bag| bag.name == "PICK" && bag.owner.as_deref() == Some(of))
        .expect("each player has a pick bag");
    (bag.pieces.keys().cloned().collect(), bag.hidden, bag.count)
}

fn winner(session: &Session) -> Option<String> {
    match session.view().status {
        Status::Won { winner } => Some(winner),
        _ => None,
    }
}

#[test]
fn a_round_asks_both_players_at_once_and_each_sees_only_its_own_choice() {
    let mut session = sealed();
    assert_eq!(session.view_for(&seat("WHITE")).turn, "WHITE");
    round(&mut session);
    // The asker is asked last: BLACK first, WHITE next; both owe a throw now.
    for who in ["WHITE", "BLACK"] {
        let view = session.view_for(&seat(who));
        assert_eq!(view.waiting, ["BLACK", "WHITE"], "{who}");
        assert_eq!(view.turn, who, "each looks at its own turn");
        let mut hands = view.choice.codes.clone();
        hands.sort();
        assert_eq!(hands, ["PAPER", "ROCK", "SCISSORS"], "{who}");
        assert!(session.is_to_move(who));
    }
    // A spectator is told who is awaited and has nothing to choose.
    let onlooker = session.view_for(&Viewer::Spectator);
    assert_eq!(onlooker.waiting, ["BLACK", "WHITE"]);
    assert!(onlooker.choice.codes.is_empty());
    assert!(!session.is_to_move("NOBODY"));
}

#[test]
fn a_throw_is_kept_from_the_other_player_until_both_are_in() {
    let mut session = sealed();
    round(&mut session);
    throw(&mut session, "BLACK", "ROCK");
    // BLACK has answered: it is told so, and has nothing more to choose.
    let black = session.view_for(&seat("BLACK"));
    assert_eq!(black.waiting, ["WHITE"]);
    assert!(black.choice.codes.is_empty() && black.choice.actions.is_empty());
    assert!(!session.is_to_move("BLACK"));
    // WHITE sees that BLACK has thrown, and not what.
    let white = session.view_for(&seat("WHITE"));
    assert_eq!(white.waiting, ["WHITE"]);
    assert_eq!(white.turn, "WHITE");
    assert!(white.choice.codes.len() == 3, "{:?}", white.choice);
    assert!(matches!(white.status, Status::Playing));
    // The pick shows nowhere: the board is empty and the history holds only the round.
    assert!(white.pieces.is_empty());
    let (hands, hidden, _) = pick(&white, "BLACK");
    assert!(
        hands.is_empty() && hidden,
        "BLACK's pick is hidden from WHITE"
    );
    let history = session.history_for(&seat("WHITE"));
    assert_eq!(history.len(), 1, "a kept throw is not in the history yet");
}

#[test]
fn a_thrower_sees_its_own_kept_throw_and_the_other_does_not() {
    let mut session = sealed();
    round(&mut session);
    throw(&mut session, "BLACK", "ROCK");
    let black = session.view_for(&seat("BLACK"));
    let (hands, hidden, _) = pick(&black, "BLACK");
    assert_eq!(hands, ["ROCK"], "BLACK sees what it threw");
    assert!(!hidden);
    let white = session.view_for(&seat("WHITE"));
    let (hands, hidden, _) = pick(&white, "BLACK");
    assert!(hands.is_empty() && hidden, "WHITE does not");
    let onlooker = session.view_for(&Viewer::Spectator);
    assert!(pick(&onlooker, "BLACK").0.is_empty());
}

#[test]
fn both_throws_are_played_together_and_the_better_wins_in_either_order() {
    for first in ["BLACK", "WHITE"] {
        let mut session = sealed();
        round(&mut session);
        let second = if first == "BLACK" { "WHITE" } else { "BLACK" };
        // BLACK throws rock and WHITE scissors, whichever of them throws first.
        let hands = |who: &str| if who == "BLACK" { "ROCK" } else { "SCISSORS" };
        throw(&mut session, first, hands(first));
        assert!(
            matches!(session.view().status, Status::Playing),
            "{first} first"
        );
        throw(&mut session, second, hands(second));
        assert_eq!(winner(&session).as_deref(), Some("BLACK"), "{first} first");
        // The match is over, so nothing is hidden: both picks are shown, and the history says who
        // threw what, BLACK first, as the seats were asked.
        let view = session.view_for(&seat("WHITE"));
        assert_eq!(pick(&view, "BLACK").0, ["ROCK"]);
        assert_eq!(pick(&view, "WHITE").0, ["SCISSORS"]);
        let history = session.history_for(&seat("WHITE"));
        let thrown: Vec<(String, String)> = history
            .iter()
            .map(|record| (record.player.clone(), record.action.clone()))
            .collect();
        assert_eq!(
            thrown,
            [("WHITE", "PLAY"), ("BLACK", "THROW"), ("WHITE", "THROW")]
                .map(|(player, action)| (player.to_string(), action.to_string())),
            "{first} first"
        );
        assert!(view.waiting.is_empty());
    }
}

#[test]
fn every_pair_of_throws_is_judged_and_a_draw_is_played_again() {
    let beats = [
        ("ROCK", "SCISSORS"),
        ("PAPER", "ROCK"),
        ("SCISSORS", "PAPER"),
    ];
    for (white, black) in [
        ("ROCK", "SCISSORS"),
        ("SCISSORS", "ROCK"),
        ("PAPER", "ROCK"),
        ("ROCK", "PAPER"),
        ("SCISSORS", "PAPER"),
        ("PAPER", "SCISSORS"),
    ] {
        let mut session = sealed();
        round(&mut session);
        throw(&mut session, "WHITE", white);
        throw(&mut session, "BLACK", black);
        let expected = if beats.contains(&(white, black)) {
            "WHITE"
        } else {
            "BLACK"
        };
        assert_eq!(
            winner(&session).as_deref(),
            Some(expected),
            "{white} against {black}"
        );
    }
    // A draw: the match goes on, with the other player to start the next round.
    let mut session = sealed();
    round(&mut session);
    throw(&mut session, "WHITE", "PAPER");
    throw(&mut session, "BLACK", "PAPER");
    assert!(matches!(session.view().status, Status::Playing));
    assert!(session.view().waiting.is_empty());
    assert_eq!(session.view_for(&seat("BLACK")).turn, "BLACK");
    assert!(!session.is_to_move("WHITE"));
    round_by(&mut session, "BLACK");
    assert_eq!(session.view().waiting, ["WHITE", "BLACK"]);
    // The last round's throws are gone; both throw anew.
    assert_eq!(pick(&session.view_for(&seat("BLACK")), "BLACK").0.len(), 0);
    throw(&mut session, "BLACK", "ROCK");
    throw(&mut session, "WHITE", "PAPER");
    assert_eq!(winner(&session).as_deref(), Some("WHITE"));
}

fn round_by(session: &mut Session, who: &str) {
    session
        .choose_as(Some(who), Answer::Action("PLAY".to_string()))
        .unwrap();
}

#[test]
fn a_player_throws_once_and_only_when_asked() {
    let mut session = sealed();
    // Before the round there is nothing to throw.
    assert!(session
        .choose_as(Some("BLACK"), Answer::Code("ROCK".into()))
        .is_err());
    round(&mut session);
    assert!(session
        .choose_as(Some("NOBODY"), Answer::Code("ROCK".into()))
        .is_err());
    assert!(session
        .choose_as(None, Answer::Code("ROCK".into()))
        .is_err());
    assert!(session
        .choose_as(Some("BLACK"), Answer::Code("LIZARD".into()))
        .is_err());
    throw(&mut session, "BLACK", "ROCK");
    assert!(
        session
            .choose_as(Some("BLACK"), Answer::Code("PAPER".into()))
            .is_err(),
        "a throw is kept, and is not changed"
    );
    assert!(session.cancel_as(Some("BLACK")).is_err());
    // The round cannot be started again while it is on.
    assert!(session
        .choose_as(Some("WHITE"), Answer::Action("PLAY".into()))
        .is_err());
}

#[test]
fn a_match_saved_part_way_resumes_with_the_throws_kept_and_the_rest_to_come() {
    let mut session = sealed();
    round(&mut session);
    throw(&mut session, "BLACK", "SCISSORS");
    let saved = session.state();
    let mut resumed = Session::resume(RPS, &saved, session.revision()).expect("it resumes");
    assert_eq!(resumed.view_for(&seat("WHITE")).waiting, ["WHITE"]);
    assert!(!resumed.is_to_move("BLACK"));
    throw(&mut resumed, "WHITE", "ROCK");
    assert_eq!(winner(&resumed).as_deref(), Some("WHITE"));
    // A saved state whose kept throw is for a seat the rules lack is not resumed.
    let mut tampered = saved.clone();
    tampered["together"]["seats"][0]["player"] = json!("NOBODY");
    assert!(Session::resume(RPS, &tampered, 0).is_err());
}

#[test]
fn in_the_open_a_throw_takes_effect_as_it_comes_and_the_other_player_sees_it() {
    let mut session = open();
    round(&mut session);
    throw(&mut session, "BLACK", "PAPER");
    let white = session.view_for(&seat("WHITE"));
    assert_eq!(
        pick(&white, "BLACK").0,
        ["PAPER"],
        "WHITE sees BLACK's throw"
    );
    let history = session.history_for(&seat("WHITE"));
    assert_eq!(history.len(), 2, "and the throw is in the history at once");
    assert_eq!(white.waiting, ["WHITE"]);
    assert!(matches!(white.status, Status::Playing));
    throw(&mut session, "WHITE", "SCISSORS");
    assert_eq!(winner(&session).as_deref(), Some("WHITE"));
}

#[test]
fn one_person_at_a_hot_seat_throws_for_each_seat_in_turn() {
    let mut session = sealed();
    round(&mut session);
    let view = session.view();
    assert_eq!(view.turn, "BLACK", "the first seat asked");
    assert_eq!(view.choice.codes.len(), 3);
    session.choose(Answer::Code("ROCK".into())).unwrap();
    assert_eq!(session.view().turn, "WHITE");
    session.choose(Answer::Code("PAPER".into())).unwrap();
    assert_eq!(winner(&session).as_deref(), Some("WHITE"));
}

/// A whole game of random throws, saved and resumed after every answer, and the same game played
/// straight through, must agree.
#[test]
fn random_matches_end_and_resume_at_every_step() {
    let hands = ["ROCK", "PAPER", "SCISSORS"];
    let (mut wins, mut draws_replayed) = (0, 0);
    for seed in 0..30u64 {
        let mut dice = Dice(seed + 3);
        let mut straight = sealed();
        let mut resumed = sealed();
        let mut rounds = 0;
        while matches!(straight.view().status, Status::Playing) && rounds < 40 {
            rounds += 1;
            let starter = straight.view().turn;
            for session in [&mut straight, &mut resumed] {
                session
                    .choose_as(Some(&starter), Answer::Action("PLAY".into()))
                    .unwrap();
            }
            let mut order = vec!["WHITE", "BLACK"];
            if dice.below(2) == 1 {
                order.reverse();
            }
            for who in order {
                let hand = hands[dice.below(3)];
                throw(&mut straight, who, hand);
                throw(&mut resumed, who, hand);
                // Save and resume the second game after every answer.
                let saved = resumed.state();
                resumed = Session::resume(RPS, &saved, resumed.revision()).expect("it resumes");
                assert_eq!(straight.state(), resumed.state(), "seed {seed}");
            }
            if matches!(straight.view().status, Status::Playing) {
                draws_replayed += 1;
            }
        }
        assert!(winner(&straight).is_some(), "seed {seed} ends");
        assert_eq!(winner(&straight), winner(&resumed));
        wins += 1;
    }
    assert_eq!(wins, 30);
    assert!(draws_replayed > 0, "some round was drawn and played again");
}
