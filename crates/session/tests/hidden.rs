//! Hidden information through the session: a view per viewer, the reveal at game over, the
//! derived numbers withheld, the history redacted, and a property test that no viewer's view
//! ever holds a card they may not see.

mod support;

use std::collections::BTreeSet;

use modern_chess_session::{Answer, Session, View, Viewer};
use serde_json::Value;
use support::{step, Dice};

const SECRETS: &str = include_str!("fixtures/secrets.json");
const CHESS: &str = include_str!("../../engine/specs/chess.json");
const RACE: &str = include_str!("fixtures/dice_race.json");

fn white() -> Viewer {
    Viewer::Player("WHITE".to_string())
}

fn black() -> Viewer {
    Viewer::Player("BLACK".to_string())
}

fn session(seed: u64) -> Session {
    Session::new_with_seed(SECRETS, seed).expect("the secrets game reads")
}

/// The game without `CRAFT`, whose choice lists the names of cards that go into a hand: names
/// that are public by design, and that the property below would take for the cards.
fn session_without_crafting(seed: u64) -> Session {
    let mut spec: Value = serde_json::from_str(SECRETS).unwrap();
    spec["actions"]
        .as_array_mut()
        .unwrap()
        .retain(|action| action["name"] != "CRAFT");
    Session::new_with_seed(&spec.to_string(), seed).expect("the game reads")
}

fn take(session: &mut Session, action: &str) {
    session
        .choose(Answer::Action(action.to_string()))
        .unwrap_or_else(|error| panic!("{action} is refused: {error}"));
}

fn bag<'a>(view: &'a View, name: &str, owner: Option<&str>) -> &'a modern_chess_session::BagView {
    view.bags
        .iter()
        .find(|bag| bag.name == name && bag.owner.as_deref() == owner)
        .unwrap_or_else(|| panic!("no bag {name} of {owner:?}"))
}

fn total(bag: &modern_chess_session::BagView) -> u32 {
    bag.pieces.values().sum()
}

/// Draws until the deck is empty, which ends the game (the end check reads the deck).
fn play_out(session: &mut Session) {
    for _ in 0..40 {
        if session.view().choice.actions.is_empty() && session.view().choice.held.is_empty() {
            break;
        }
        take(session, "BURN");
    }
}

// ---- nothing hidden, nothing changed

#[test]
fn a_game_that_hides_nothing_has_the_same_view_for_everyone() {
    for (spec, seed) in [(CHESS, 0u64), (RACE, 4)] {
        let session = Session::new_with_seed(spec, seed).unwrap();
        let full = serde_json::to_value(session.view()).unwrap();
        let view = session.view();
        let others = view.players.iter().find(|name| **name != view.turn).unwrap();
        let viewers = [
            (Viewer::Player(view.turn.clone()), true),
            (Viewer::Player(others.clone()), false),
            (Viewer::Spectator, false),
            (Viewer::Everyone, true),
        ];
        for (viewer, is_mover) in viewers {
            let mut seen = serde_json::to_value(session.view_for(&viewer)).unwrap();
            // Only the mover has a choice; the rest of the view is the same for every viewer.
            if !is_mover {
                seen["choice"] = full["choice"].clone();
            }
            assert_eq!(seen, full, "{viewer:?}");
        }
        let text = full.to_string();
        assert!(!text.contains("hidden") && !text.contains("\"count\":"), "{text}");
    }
}

#[test]
fn view_is_the_view_for_everyone() {
    let mut session = session(1);
    take(&mut session, "DRAW");
    assert_eq!(session.view(), session.view_for(&Viewer::Everyone));
}

#[test]
fn everyone_sees_everything_as_a_hot_seat_does() {
    let mut session = session(1);
    take(&mut session, "DRAW");
    let view = session.view();
    assert_eq!(total(bag(&view, "DECK", None)), 5);
    assert!(!bag(&view, "DECK", None).hidden);
    assert_eq!(total(bag(&view, "HAND", Some("WHITE"))), 1);
}

// ---- who sees what

#[test]
fn a_seat_sees_its_own_hand_and_not_the_others() {
    let mut session = session(1);
    take(&mut session, "DRAW"); // WHITE draws
    take(&mut session, "DRAW"); // BLACK draws
    let mine = session.view_for(&white());
    assert!(!bag(&mine, "HAND", Some("WHITE")).hidden);
    assert_eq!(total(bag(&mine, "HAND", Some("WHITE"))), 1);
    let theirs = bag(&mine, "HAND", Some("BLACK"));
    assert!(theirs.hidden && theirs.pieces.is_empty());
    assert_eq!(theirs.count, Some(1), "a hand's size is public");
    // And the other way round.
    let other = session.view_for(&black());
    assert!(bag(&other, "HAND", Some("WHITE")).hidden);
    assert_eq!(total(bag(&other, "HAND", Some("BLACK"))), 1);
}

#[test]
fn a_bag_nobody_sees_is_hidden_from_every_seat_with_its_count() {
    let mut session = session(1);
    take(&mut session, "DRAW");
    for viewer in [white(), black(), Viewer::Spectator] {
        let view = session.view_for(&viewer);
        let deck = bag(&view, "DECK", None);
        assert!(deck.hidden && deck.pieces.is_empty(), "{viewer:?}");
        assert_eq!(deck.count, Some(5), "{viewer:?}");
    }
}

#[test]
fn a_spectator_sees_no_hand() {
    let mut session = session(1);
    take(&mut session, "DRAW");
    take(&mut session, "DRAW");
    let view = session.view_for(&Viewer::Spectator);
    for player in ["WHITE", "BLACK"] {
        assert!(bag(&view, "HAND", Some(player)).hidden);
    }
}

#[test]
fn a_public_bag_is_seen_by_everyone() {
    let mut session = session(1);
    take(&mut session, "BURN");
    take(&mut session, "BURN");
    for viewer in [white(), black(), Viewer::Spectator, Viewer::Everyone] {
        let view = session.view_for(&viewer);
        let row = bag(&view, "ROW", None);
        assert!(!row.hidden && total(row) == 2, "{viewer:?}");
    }
}

#[test]
fn the_count_is_left_out_when_the_bag_does_not_show_it() {
    let mut spec: Value = serde_json::from_str(SECRETS).unwrap();
    spec["bags"][0]["shows_count"] = Value::Bool(false);
    let session = Session::new_with_seed(&spec.to_string(), 1).unwrap();
    let view = session.view_for(&white());
    let deck = bag(&view, "DECK", None);
    assert!(deck.hidden && deck.count.is_none());
    let text = serde_json::to_value(&view).unwrap()["bags"][0].to_string();
    assert!(!text.contains("count"), "{text}");
}

#[test]
fn the_hidden_marks_show_in_the_json_only_where_something_is_hidden() {
    let session = session(1);
    let view = serde_json::to_value(session.view_for(&white())).unwrap();
    let bags = view["bags"].as_array().unwrap();
    let deck = bags.iter().find(|bag| bag["name"] == "DECK").unwrap();
    assert_eq!(deck["hidden"], true);
    assert_eq!(deck["count"], 6);
    let row = bags.iter().find(|bag| bag["name"] == "ROW").unwrap();
    assert!(row.get("hidden").is_none() && row.get("count").is_none(), "{row}");
}

// ---- the end of the game shows everything

#[test]
fn when_the_game_is_over_every_bag_is_shown_to_everyone() {
    let mut session = session(3);
    take(&mut session, "DRAW");
    take(&mut session, "DRAW");
    assert!(bag(&session.view_for(&white()), "HAND", Some("BLACK")).hidden);
    play_out(&mut session);
    assert!(matches!(
        session.view().status,
        modern_chess_session::Status::Draw
    ));
    for viewer in [white(), black(), Viewer::Spectator] {
        let view = session.view_for(&viewer);
        assert!(view.bags.iter().all(|bag| !bag.hidden), "{viewer:?}");
        assert_eq!(view, session.view(), "{viewer:?} sees what everyone does");
    }
}

// ---- the choice

#[test]
fn only_the_mover_has_a_choice_and_it_names_only_their_own_cards() {
    let mut session = session(1);
    take(&mut session, "DRAW"); // WHITE holds a card, BLACK to move
    take(&mut session, "DRAW"); // BLACK holds one, WHITE to move
    assert_eq!(session.view().turn, "WHITE");
    take(&mut session, "PLAY");
    let mover = session.view_for(&white());
    assert_eq!(mover.choice.held.len(), 1);
    let opponent = session.view_for(&black());
    assert_eq!(opponent.choice, Default::default(), "no choice for the other seat");
    let spectator = session.view_for(&Viewer::Spectator);
    assert_eq!(spectator.choice, Default::default());
}

// ---- material

#[test]
fn material_that_reads_a_hand_is_shown_only_to_who_may_see_the_hand() {
    let mut session = session(1);
    take(&mut session, "DRAW");
    take(&mut session, "DRAW");
    let everyone = session.view().material;
    assert_eq!(everyone.len(), 2);
    let mine = session.view_for(&white()).material;
    assert_eq!(mine.get("WHITE"), everyone.get("WHITE"));
    assert!(!mine.contains_key("BLACK"), "{mine:?}");
    assert!(session.view_for(&Viewer::Spectator).material.is_empty());
}

#[test]
fn material_is_shown_to_all_when_the_game_is_over() {
    let mut session = session(2);
    take(&mut session, "DRAW");
    play_out(&mut session);
    let everyone = session.view().material;
    assert_eq!(session.view_for(&Viewer::Spectator).material, everyone);
    assert_eq!(session.view_for(&white()).material, everyone);
}

// ---- the history

fn history_json(session: &Session, viewer: &Viewer) -> Vec<Value> {
    session
        .history_for(viewer)
        .iter()
        .map(|record| serde_json::to_value(record).unwrap())
        .collect()
}

/// A session where WHITE has rolled into their hand, and BLACK has drawn.
fn after_a_roll_and_a_draw() -> Session {
    let mut session = session(5);
    take(&mut session, "LUCK"); // WHITE rolls a card into their hand
    take(&mut session, "DRAW"); // BLACK draws one
    session
}

#[test]
fn a_roll_into_a_hidden_hand_shows_its_face_to_the_owner_alone() {
    let session = after_a_roll_and_a_draw();
    let faces = |viewer: &Viewer| {
        history_json(&session, viewer)[0]["rolled"][0]["face"].clone()
    };
    let rolled = faces(&white());
    assert!(rolled.as_object().is_some_and(|face| face.len() == 1), "{rolled}");
    assert_eq!(faces(&Viewer::Everyone), rolled);
    for viewer in [black(), Viewer::Spectator] {
        let blank = faces(&viewer);
        assert!(blank.as_object().is_some_and(|face| face.is_empty()), "{viewer:?} {blank}");
        // The die was rolled: that much shows.
        assert_eq!(history_json(&session, &viewer)[0]["rolled"].as_array().unwrap().len(), 1);
    }
}

#[test]
fn a_card_played_from_a_hidden_hand_is_blanked_for_the_other_seat() {
    let mut session = session(5);
    take(&mut session, "LUCK"); // WHITE
    take(&mut session, "LUCK"); // BLACK
    take(&mut session, "PLAY"); // WHITE
    let held = session.view().choice.held[0].clone();
    session.choose(Answer::Held(held.clone())).unwrap();
    let played = |viewer: &Viewer| history_json(&session, viewer)[2].clone();
    let own = played(&white());
    assert_eq!(own["piece_code"], held);
    assert_eq!(own["chosen"].to_string().matches(&held).count(), 1, "{own}");
    let other = played(&black());
    assert_eq!(other["piece_code"], "");
    assert!(!other.to_string().contains(&held), "{other}");
    assert_eq!(played(&Viewer::Everyone), own);
}

#[test]
fn a_whole_history_is_shown_to_everyone_once_the_game_is_over() {
    let mut session = after_a_roll_and_a_draw();
    play_out(&mut session);
    let full = history_json(&session, &Viewer::Everyone);
    for viewer in [white(), black(), Viewer::Spectator] {
        assert_eq!(history_json(&session, &viewer), full, "{viewer:?}");
    }
}

#[test]
fn a_shuffle_is_recorded_as_a_name_and_never_an_order() {
    let mut session = session(5);
    take(&mut session, "DRAW");
    let record = &history_json(&session, &black())[0];
    assert_eq!(record["shuffled"], serde_json::json!(["DECK"]));
}

// ---- errors do not tell

#[test]
fn an_answer_that_fails_because_of_a_hand_fails_like_any_other() {
    let mut session = session(1);
    take(&mut session, "LUCK"); // WHITE holds a card
    take(&mut session, "LUCK"); // BLACK holds one; WHITE to move
    take(&mut session, "PLAY");
    let blacks = session
        .view_for(&Viewer::Everyone)
        .bags
        .iter()
        .find(|bag| bag.owner.as_deref() == Some("BLACK") && bag.name == "HAND")
        .and_then(|bag| bag.pieces.keys().next().cloned())
        .expect("black holds a card");
    let before = session.state();
    let theirs = session.choose(Answer::Held(blacks.clone())).unwrap_err();
    let nothing = session.choose(Answer::Held("NOPE".to_string())).unwrap_err();
    assert_eq!(theirs, nothing, "a card in the other hand is told like a card that is nowhere");
    assert!(!theirs.contains(&blacks), "{theirs}");
    assert_eq!(session.state(), before, "a refused answer changes nothing");
}

// ---- what the review found

/// WHITE holds a card and is part way through putting it on the board.
fn white_halfway_through_a_table() -> (Session, String) {
    let mut session = session(5);
    take(&mut session, "LUCK"); // WHITE holds a card
    take(&mut session, "BURN"); // BLACK
    take(&mut session, "TABLE");
    let held = session.view().choice.held[0].clone();
    session.choose(Answer::Held(held.clone())).unwrap();
    session.choose(Answer::Place("0,0".into())).unwrap();
    (session, held)
}

#[test]
fn the_other_seat_is_not_shown_a_card_the_mover_is_part_way_through_playing() {
    let (session, held) = white_halfway_through_a_table();
    // The mover may still take it back, and it came out of a hand the others cannot see.
    let mover = session.view_for(&white());
    assert!(mover.pieces.iter().any(|piece| piece.code == held), "the mover sees what they chose");
    for viewer in [black(), Viewer::Spectator] {
        let view = session.view_for(&viewer);
        assert!(view.pieces.is_empty(), "{viewer:?} was shown {:?}", view.pieces);
    }
    assert_eq!(session.view().pieces, mover.pieces, "a hot seat plays every side");
}

#[test]
fn a_game_that_hides_nothing_still_shows_everyone_the_turn_so_far() {
    let mut spec: Value = serde_json::from_str(SECRETS).unwrap();
    for bag in spec["bags"].as_array_mut().unwrap() {
        let object = bag.as_object_mut().unwrap();
        for key in ["visible", "shows_count", "strict"] {
            object.remove(key);
        }
    }
    let mut session = Session::new_with_seed(&spec.to_string(), 5).unwrap();
    take(&mut session, "LUCK");
    take(&mut session, "BURN");
    take(&mut session, "TABLE");
    let held = session.view().choice.held[0].clone();
    session.choose(Answer::Held(held)).unwrap();
    session.choose(Answer::Place("0,0".into())).unwrap();
    assert_eq!(session.view_for(&black()).pieces, session.view().pieces);
    assert_eq!(session.view_for(&black()).pieces.len(), 1);
}

#[test]
fn every_held_answer_of_a_turn_is_blanked_when_any_bag_it_touched_is_hidden() {
    let mut session = session(5);
    take(&mut session, "LUCK"); // WHITE holds a card
    take(&mut session, "LUCK"); // BLACK holds one
    take(&mut session, "BURN"); // WHITE: puts a card in the (public) row, for the swap
    take(&mut session, "LUCK"); // BLACK
    take(&mut session, "SWAP"); // WHITE: a card from the hand, then one from the row
    let from_hand = session.view().choice.held[0].clone();
    session.choose(Answer::Held(from_hand.clone())).unwrap();
    let from_row = session.view().choice.held[0].clone();
    session.choose(Answer::Held(from_row)).unwrap();
    let record = &history_json(&session, &black())[4];
    assert_eq!(record["action"], "SWAP");
    let shown = record["chosen"].to_string();
    assert!(!shown.contains(&from_hand), "{shown}");
    assert_eq!(record["piece_code"], "");
    // The mover remembers what they played.
    assert!(history_json(&session, &white())[4]["chosen"].to_string().contains(&from_hand));
}

#[test]
fn a_code_chosen_for_a_hidden_hand_is_blanked_for_the_other_seat() {
    let mut session = session(5);
    take(&mut session, "CRAFT");
    session.choose(Answer::Code("XR2".to_string())).unwrap();
    let theirs = history_json(&session, &black())[0]["chosen"].to_string();
    assert!(!theirs.contains("XR2"), "{theirs}");
    let own = history_json(&session, &white())[0]["chosen"].to_string();
    assert!(own.contains("XR2"), "{own}");
}

#[test]
fn a_history_of_a_game_that_hides_nothing_has_no_new_keys() {
    let mut spec: Value = serde_json::from_str(SECRETS).unwrap();
    for bag in spec["bags"].as_array_mut().unwrap() {
        let object = bag.as_object_mut().unwrap();
        for key in ["visible", "shows_count", "strict"] {
            object.remove(key);
        }
    }
    let mut session = Session::new_with_seed(&spec.to_string(), 5).unwrap();
    take(&mut session, "LUCK");
    take(&mut session, "LUCK");
    take(&mut session, "PLAY");
    let held = session.view().choice.held[0].clone();
    session.choose(Answer::Held(held)).unwrap();
    for record in history_json(&session, &Viewer::Everyone) {
        assert!(record.get("private_bags").is_none(), "{record}");
    }
    let saved = session.state()["history"].to_string();
    assert!(!saved.contains("private_bags"), "{saved}");
}

// ---- the property: no view holds a card its viewer may not see

/// The codes in the bags `viewer` may not see, and the codes in the ones they may.
fn codes_by_sight(session: &Session, viewer: &Viewer) -> (BTreeSet<String>, BTreeSet<String>) {
    let (mut hidden, mut seen) = (BTreeSet::new(), BTreeSet::new());
    for bag in &session.view().bags {
        let sees = match (bag.name.as_str(), viewer) {
            (_, Viewer::Everyone) => true,
            ("HAND", Viewer::Player(name)) => bag.owner.as_deref() == Some(name.as_str()),
            ("ROW", _) => true,
            _ => false,
        };
        let codes = bag.pieces.keys().cloned();
        if sees {
            seen.extend(codes);
        } else {
            hidden.extend(codes);
        }
    }
    // A card put on the board is there for everyone (the board as it stands, not a turn in
    // the making, which only the mover is shown).
    let board = session.view_for(&Viewer::Spectator).pieces;
    seen.extend(board.iter().map(|piece| piece.code.clone()));
    (hidden, seen)
}

#[test]
fn no_view_ever_holds_a_card_its_viewer_may_not_see() {
    let viewers = [white(), black(), Viewer::Spectator];
    for seed in 0..60u64 {
        let mut dice = Dice(seed.wrapping_mul(7919) + 1);
        let mut session = session_without_crafting(seed);
        // What each viewer has been shown so far: a history may keep a card that was in view.
        let mut ever_seen: Vec<BTreeSet<String>> = vec![BTreeSet::new(); viewers.len()];
        for round in 0..120 {
            let over = !matches!(session.view().status, modern_chess_session::Status::Playing);
            for (who, viewer) in viewers.iter().enumerate() {
                let text = serde_json::to_string(&session.view_for(viewer)).unwrap();
                let history = serde_json::to_string(&session.history_for(viewer)).unwrap();
                let (hidden, seen) = codes_by_sight(&session, viewer);
                ever_seen[who].extend(seen.iter().cloned());
                // A code can be both: a roll deals a card another bag already holds.
                let hidden: BTreeSet<String> = hidden.difference(&seen).cloned().collect();
                if !over {
                    for code in &hidden {
                        assert!(!text.contains(code), "seed {seed} round {round}: {viewer:?} was shown {code}\n{text}");
                    }
                }
                for code in &seen {
                    // What a viewer may see is there to be seen (the history shows only what
                    // happened, so only the view is held to this).
                    assert!(text.contains(code), "seed {seed} round {round}: {viewer:?} lost {code}");
                }
                if !over {
                    // A card the viewer may not see is not in the history either, unless it is a
                    // card that someone showed by playing it where everyone sees it.
                    for code in hidden.difference(&ever_seen[who]) {
                        assert!(!history.contains(code), "seed {seed} round {round}: {viewer:?} history shows {code}\n{history}");
                    }
                }
            }
            if over {
                break;
            }
            step(&mut session, &mut dice);
        }
    }
}
