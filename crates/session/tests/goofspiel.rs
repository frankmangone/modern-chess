//! Goofspiel, the Game of Pure Strategy: a shuffled deck of prizes, a hand of bidding cards for each
//! player that anyone may see, and a bid made in secret by both players at once. The higher bid takes
//! the prize and a tie throws it out. This proves what the book promises of hidden information on a
//! whole game: the bid is hidden until both are in, the deck is hidden always, and a round hands the
//! turn to the other player.

use modern_chess_session::{Answer, BagView, Session, Status, View, Viewer};

const GOOFSPIEL: &str = include_str!("../../engine/specs/goofspiel.json");
const RANKS: u32 = 13;

fn code(rank: u32) -> String {
    format!("C{rank:02}")
}

fn seat(name: &str) -> Viewer {
    Viewer::Player(name.to_string())
}

fn game() -> Session {
    Session::new(GOOFSPIEL).expect("goofspiel reads")
}

fn bag<'a>(view: &'a View, name: &str, owner: Option<&str>) -> &'a BagView {
    view.bags
        .iter()
        .find(|bag| bag.name == name && bag.owner.as_deref() == owner)
        .unwrap_or_else(|| panic!("no bag {name} of {owner:?}"))
}

/// The ranks a bag holds, as the viewer sees them, in order.
fn ranks_in(view: &View, name: &str, owner: Option<&str>) -> Vec<u32> {
    bag(view, name, owner)
        .pieces
        .keys()
        .map(|key| key[1..].parse().unwrap())
        .collect()
}

fn start_round(session: &mut Session) {
    session.choose(Answer::Action("ROUND".to_string())).unwrap();
}

fn bid(session: &mut Session, who: &str, rank: u32) {
    session
        .choose_as(Some(who), Answer::Held(code(rank)))
        .unwrap_or_else(|error| panic!("{who} cannot bid {rank}: {error}"));
}

fn reveal(session: &mut Session) {
    session
        .choose(Answer::Action("REVEAL".to_string()))
        .unwrap();
}

/// A whole round: both bid, and the player whose turn it is turns the bids over.
fn play_round(session: &mut Session, red: u32, blue: u32) {
    start_round(session);
    bid(session, "RED", red);
    bid(session, "BLUE", blue);
    reveal(session);
}

#[test]
fn a_new_game_has_thirteen_cards_in_each_hand_and_a_deck_nobody_sees() {
    let session = game();
    let view = session.view_for(&seat("RED"));
    assert_eq!(
        ranks_in(&view, "HAND", Some("RED")),
        (1..=RANKS).collect::<Vec<_>>()
    );
    assert_eq!(
        ranks_in(&view, "HAND", Some("BLUE")),
        (1..=RANKS).collect::<Vec<_>>(),
        "hands are open"
    );
    let deck = bag(&view, "DECK", None);
    assert!(deck.hidden && deck.pieces.is_empty());
    assert_eq!(
        deck.count,
        Some(RANKS),
        "everyone is told how many prizes are left"
    );
    assert!(bag(&view, "PRIZE", None).pieces.is_empty());
    assert_eq!(view.turn, "RED");
}

#[test]
fn a_round_turns_a_prize_over_and_asks_both_players_at_once() {
    let mut session = game();
    start_round(&mut session);
    let view = session.view_for(&seat("BLUE"));
    assert_eq!(bag(&view, "DECK", None).count, Some(RANKS - 1));
    assert_eq!(
        ranks_in(&view, "PRIZE", None).len(),
        1,
        "one prize is on offer"
    );
    assert_eq!(view.waiting, ["BLUE", "RED"], "the asker is asked last");
    for who in ["RED", "BLUE"] {
        assert!(session.is_to_move(who), "{who} owes a bid");
        let mut held = session.view_for(&seat(who)).choice.held;
        held.sort();
        assert_eq!(
            held,
            (1..=RANKS).map(code).collect::<Vec<_>>(),
            "{who} may bid any card in hand"
        );
    }
}

#[test]
fn a_bid_is_hidden_from_the_other_player_until_both_are_in_and_then_shown_to_both() {
    let mut session = game();
    start_round(&mut session);
    bid(&mut session, "BLUE", 9);
    // BLUE has bid and is told so; RED sees that BLUE has bid, and not what.
    let red = session.view_for(&seat("RED"));
    assert_eq!(red.waiting, ["RED"]);
    let blue_pick = bag(&red, "PICK", Some("BLUE"));
    assert!(blue_pick.hidden && blue_pick.pieces.is_empty());
    assert_eq!(
        red.waiting,
        ["RED"],
        "RED is told BLUE has bid by who is still awaited, not by the bag"
    );
    // BLUE has nothing more to choose, and is shown what it did: the card is out of its hand and
    // lies face down in its bid.
    let blue = session.view_for(&seat("BLUE"));
    assert!(blue.choice.held.is_empty() && blue.choice.actions.is_empty());
    assert_eq!(blue.waiting, ["RED"]);
    assert_eq!(ranks_in(&blue, "PICK", Some("BLUE")), [9]);
    assert!(!bag(&blue, "PICK", Some("BLUE")).hidden);
    assert!(!ranks_in(&blue, "HAND", Some("BLUE")).contains(&9));
    assert_eq!(ranks_in(&blue, "HAND", Some("BLUE")).len(), 12);
    // RED is not shown it: BLUE's hand still holds all thirteen for RED, and its bid is empty.
    assert!(ranks_in(&red, "HAND", Some("BLUE")).contains(&9));
    assert_eq!(ranks_in(&red, "HAND", Some("BLUE")).len(), 13);
    // A spectator sees neither the bid nor the card gone from the hand.
    let onlooker = session.view_for(&Viewer::Spectator);
    assert!(bag(&onlooker, "PICK", Some("BLUE")).hidden);
    assert!(ranks_in(&onlooker, "HAND", Some("BLUE")).contains(&9));

    bid(&mut session, "RED", 4);
    // Both are in. RED, who started the round, turns them over: nobody can act but RED.
    assert!(session.is_to_move("RED") && !session.is_to_move("BLUE"));
    let actions: Vec<String> = session
        .view_for(&seat("RED"))
        .choice
        .actions
        .iter()
        .map(|action| action.name.clone())
        .collect();
    assert_eq!(actions, ["REVEAL"]);
    // RED still cannot see BLUE's bid before it is turned over.
    assert!(bag(&session.view_for(&seat("RED")), "PICK", Some("BLUE")).hidden);

    reveal(&mut session);
    for viewer in [seat("RED"), seat("BLUE"), Viewer::Spectator] {
        let view = session.view_for(&viewer);
        assert_eq!(
            ranks_in(&view, "LAST", Some("BLUE")),
            [9],
            "the bids are face up now"
        );
        assert_eq!(ranks_in(&view, "LAST", Some("RED")), [4]);
        assert!(ranks_in(&view, "PICK", Some("BLUE")).is_empty());
        assert!(ranks_in(&view, "PICK", Some("RED")).is_empty());
    }
}

#[test]
fn the_higher_bid_takes_the_prize_and_it_counts_its_rank() {
    let mut session = game();
    start_round(&mut session);
    let prize = ranks_in(&session.view(), "PRIZE", None)[0];
    bid(&mut session, "RED", 13);
    bid(&mut session, "BLUE", 2);
    reveal(&mut session);
    let view = session.view_for(&seat("BLUE"));
    assert_eq!(ranks_in(&view, "WON", Some("RED")), [prize]);
    assert!(ranks_in(&view, "WON", Some("BLUE")).is_empty());
    assert!(
        ranks_in(&view, "PRIZE", None).is_empty(),
        "the prize left the table"
    );
    assert_eq!(view.score["RED"], f64::from(prize));
    assert_eq!(view.score["BLUE"], 0.0);
}

#[test]
fn the_lower_bidder_loses_whichever_player_started_the_round() {
    // The second round is started by BLUE, who turns the bids over; RED's higher bid still wins.
    let mut session = game();
    play_round(&mut session, 1, 1);
    assert_eq!(
        session.view().turn,
        "BLUE",
        "the turn went to the other player"
    );
    start_round(&mut session);
    let prize = ranks_in(&session.view(), "PRIZE", None)[0];
    bid(&mut session, "RED", 10);
    bid(&mut session, "BLUE", 3);
    assert!(
        session.is_to_move("BLUE") && !session.is_to_move("RED"),
        "the asker turns the bids over"
    );
    reveal(&mut session);
    assert_eq!(ranks_in(&session.view(), "WON", Some("RED")), [prize]);
}

#[test]
fn the_bids_stay_face_up_until_the_next_round_starts() {
    let mut session = game();
    play_round(&mut session, 4, 8);
    // Turned over, and left on the table: nothing clears them but the next round.
    assert_eq!(ranks_in(&session.view(), "LAST", Some("RED")), [4]);
    assert_eq!(ranks_in(&session.view(), "LAST", Some("BLUE")), [8]);
    start_round(&mut session);
    for viewer in [seat("RED"), seat("BLUE"), Viewer::Spectator] {
        let view = session.view_for(&viewer);
        assert!(ranks_in(&view, "LAST", Some("RED")).is_empty());
        assert!(ranks_in(&view, "LAST", Some("BLUE")).is_empty());
    }
}

#[test]
fn equal_bids_throw_the_prize_out() {
    let mut session = game();
    play_round(&mut session, 7, 7);
    let view = session.view();
    assert!(ranks_in(&view, "WON", Some("RED")).is_empty());
    assert!(ranks_in(&view, "WON", Some("BLUE")).is_empty());
    assert!(
        ranks_in(&view, "PRIZE", None).is_empty(),
        "a tied prize is out of the game"
    );
    assert_eq!(view.score["RED"], 0.0);
    assert_eq!(view.score["BLUE"], 0.0);
    assert!(matches!(view.status, Status::Playing));
}

#[test]
fn a_card_that_was_bid_is_not_offered_again() {
    let mut session = game();
    play_round(&mut session, 5, 6);
    start_round(&mut session);
    let held = session.view_for(&seat("RED")).choice.held;
    assert!(!held.contains(&code(5)) && held.len() == 12);
    assert!(
        session
            .choose_as(Some("RED"), Answer::Held(code(5)))
            .is_err(),
        "a spent card cannot be bid"
    );
}

#[test]
fn nobody_can_bid_a_card_it_does_not_hold_or_bid_twice() {
    let mut session = game();
    start_round(&mut session);
    assert!(session
        .choose_as(Some("RED"), Answer::Held("C99".to_string()))
        .is_err());
    bid(&mut session, "RED", 3);
    assert!(
        session
            .choose_as(Some("RED"), Answer::Held(code(4)))
            .is_err(),
        "one bid each"
    );
    assert!(
        session
            .choose_as(Some("BLUE"), Answer::Action("REVEAL".to_string()))
            .is_err(),
        "nothing is turned over before both bid"
    );
}

#[test]
fn thirteen_rounds_end_the_game_and_the_larger_total_wins() {
    // RED always bids the card one above BLUE's (the king against the ace), so every round has a
    // winner. The prizes are shuffled, so the total is checked against what each side actually took.
    let mut session = game();
    for blue in 1..=RANKS {
        let red = blue % RANKS + 1;
        play_round(&mut session, red, blue);
    }
    let view = session.view();
    let total: u32 = (1..=RANKS).sum();
    let won = |owner: &str| -> u32 { ranks_in(&view, "WON", Some(owner)).iter().sum() };
    assert_eq!(
        won("RED") + won("BLUE"),
        total,
        "every prize went to someone: no bid was equal"
    );
    assert_eq!(view.score["RED"], f64::from(won("RED")));
    assert_eq!(view.score["BLUE"], f64::from(won("BLUE")));
    let expected = if won("RED") > won("BLUE") {
        "RED"
    } else {
        "BLUE"
    };
    match &view.status {
        Status::Won { winner } => assert_eq!(winner, expected),
        other => panic!("the game should be over: {other:?}"),
    }
    for owner in ["RED", "BLUE"] {
        assert!(
            ranks_in(&view, "HAND", Some(owner)).is_empty(),
            "every card was bid"
        );
    }
}

#[test]
fn equal_totals_are_a_draw() {
    // Both bid the same card every round: every prize is thrown out and nobody scores.
    let mut session = game();
    for rank in 1..=RANKS {
        play_round(&mut session, rank, rank);
    }
    assert!(
        matches!(session.view().status, Status::Draw),
        "{:?}",
        session.view().status
    );
}

#[test]
fn the_same_seed_deals_the_same_prizes() {
    let prizes = |seed: u64| {
        let mut session = Session::new_with_seed(GOOFSPIEL, seed).unwrap();
        (0..RANKS)
            .map(|_| {
                start_round(&mut session);
                let prize = ranks_in(&session.view(), "PRIZE", None)[0];
                bid(&mut session, "RED", prize);
                bid(&mut session, "BLUE", prize);
                reveal(&mut session);
                prize
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(prizes(7), prizes(7));
    assert_ne!(prizes(7), prizes(8));
    let mut seen = prizes(7);
    seen.sort();
    assert_eq!(
        seen,
        (1..=RANKS).collect::<Vec<_>>(),
        "all thirteen prizes come up once"
    );
}
