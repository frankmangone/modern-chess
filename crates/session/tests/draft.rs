//! "Draft", the proof of hidden information: a shuffled deck nobody sees into, a row of face-up
//! cards, a hand only its owner sees (its size shown to all), a score that reads the hands and so
//! stays withheld until the game is over. Played to the end from many seeds, every view of every
//! turn checked.

use std::collections::BTreeMap;

use modern_chess_session::{Answer, BagView, Session, Status, View, Viewer};

const DRAFT: &str = include_str!("fixtures/draft.json");

/// What a card is worth in the score.
const VALUES: [(&str, u32); 3] = [("GOLD", 3), ("SILVER", 2), ("COPPER", 1)];

fn viewers() -> [Viewer; 4] {
    [
        Viewer::Player("WHITE".to_string()),
        Viewer::Player("BLACK".to_string()),
        Viewer::Spectator,
        Viewer::Everyone,
    ]
}

fn bag<'a>(view: &'a View, name: &str, owner: Option<&str>) -> &'a BagView {
    view.bags
        .iter()
        .find(|bag| bag.name == name && bag.owner.as_deref() == owner)
        .unwrap_or_else(|| panic!("no bag {name} of {owner:?}"))
}

fn worth(cards: &BTreeMap<String, u32>) -> u32 {
    VALUES
        .iter()
        .map(|(code, value)| value * cards.get(*code).copied().unwrap_or(0))
        .sum()
}

/// One turn: a card of the row the mover can take. Both actions begin with the same choice, so
/// naming one is not needed: the card decides.
fn take_a_card(session: &mut Session, pick: usize) {
    let held = session.view().choice.held.clone();
    assert!(!held.is_empty(), "the row offers a card");
    let code = held[pick % held.len()].clone();
    session.choose(Answer::Held(code)).unwrap();
}

#[test]
fn the_game_reads_and_starts_with_a_hidden_deck_and_hidden_hands() {
    let session = Session::new_with_seed(DRAFT, 1).unwrap();
    for viewer in viewers() {
        let view = session.view_for(&viewer);
        let deck = bag(&view, "DECK", None);
        let everyone = viewer == Viewer::Everyone;
        assert_eq!(deck.hidden, !everyone, "{viewer:?}");
        let cards: u32 = deck.pieces.values().sum();
        assert_eq!(if everyone { cards } else { deck.count.unwrap() }, 9);
        assert_eq!(bag(&view, "ROW", None).pieces.values().sum::<u32>(), 3);
    }
}

#[test]
fn it_is_played_to_the_end_with_nothing_leaking_on_the_way() {
    for seed in 0..40u64 {
        let mut session = Session::new_with_seed(DRAFT, seed).unwrap();
        let mut turns = 0;
        // What every card is, as the game over reveals it: the deck's and the row's at the start,
        // so each card taken by a player can be checked against what they were shown.
        while matches!(session.view().status, Status::Playing) {
            turns += 1;
            assert!(turns <= 12, "seed {seed}: the game goes on");
            let mover = session.view().turn;
            take_a_card(&mut session, seed as usize + turns);

            if !matches!(session.view().status, Status::Playing) {
                break;
            }
            for viewer in viewers() {
                let view = session.view_for(&viewer);
                let over = viewer == Viewer::Everyone;
                // The deck is never seen into, and its size is.
                let deck = bag(&view, "DECK", None);
                assert_eq!(deck.hidden, !over, "seed {seed} {viewer:?}");
                if !over {
                    assert!(deck.pieces.is_empty());
                    assert_eq!(
                        deck.count,
                        Some(9u32.saturating_sub(turns as u32)),
                        "seed {seed}"
                    );
                }
                // A hand is seen by its owner (and everyone, for `Everyone`, which is the
                // hot-seat view), and its size by all.
                for player in ["WHITE", "BLACK"] {
                    let hand = bag(&view, "HAND", Some(player));
                    let own = viewer == Viewer::Player(player.to_string());
                    assert_eq!(
                        hand.hidden,
                        !(own || over),
                        "seed {seed} {viewer:?} {player}"
                    );
                    if hand.hidden {
                        assert!(hand.pieces.is_empty());
                        assert!(hand.count.is_some());
                    }
                    // A score that reads a hand is shown only to who sees it.
                    assert_eq!(
                        view.score.contains_key(player),
                        !hand.hidden,
                        "seed {seed} {viewer:?} {player} {:?}",
                        view.score
                    );
                    if own {
                        assert_eq!(view.score[player], f64::from(worth(&hand.pieces)));
                    }
                }
                // The mover is told what they may take, and nobody else.
                assert_eq!(
                    view.choice.held.is_empty(),
                    viewer != Viewer::Player(view.turn.clone()) && !over,
                    "seed {seed} {viewer:?} mover {mover}"
                );
            }
        }

        // Everything is shown when it is over, and the score is what the hands are worth.
        let view = session.view_for(&Viewer::Spectator);
        let hands: BTreeMap<&str, u32> = ["WHITE", "BLACK"]
            .into_iter()
            .map(|player| (player, worth(&bag(&view, "HAND", Some(player)).pieces)))
            .collect();
        assert!(!bag(&view, "DECK", None).hidden, "seed {seed}");
        assert_eq!(
            bag(&view, "DECK", None).pieces.len(),
            0,
            "the deck is empty"
        );
        assert_eq!(
            hands.values().sum::<u32>(),
            20,
            "seed {seed}: every card is in a hand"
        );
        for player in ["WHITE", "BLACK"] {
            assert_eq!(view.score[player], f64::from(hands[player]), "seed {seed}");
        }
        match &view.status {
            Status::Won { winner } => {
                let other = if winner == "WHITE" { "BLACK" } else { "WHITE" };
                assert!(hands[winner.as_str()] > hands[other], "seed {seed}");
            }
            Status::Draw => assert_eq!(hands["WHITE"], hands["BLACK"], "seed {seed}"),
            Status::Playing => panic!("seed {seed}: not over"),
        }
    }
}

#[test]
fn the_same_seed_deals_the_same_game_and_another_does_not() {
    let play = |seed: u64| {
        let mut session = Session::new_with_seed(DRAFT, seed).unwrap();
        while matches!(session.view().status, Status::Playing) {
            take_a_card(&mut session, 0);
        }
        serde_json::to_value(session.view()).unwrap()
    };
    assert_eq!(play(3), play(3));
    assert!((4..20).any(|seed| play(seed) != play(3)));
}
