//! Snakes and Ladders through the session: the view the players are shown (a map of 101 squares,
//! two tokens, one action), a game played to its end with the match saved and resumed after every
//! roll, and what a client is told of the dice.

use modern_chess_session::{Answer, Session, Status, View};

const GAME: &str = include_str!("../../engine/specs/snakes_and_ladders.json");

fn token(view: &View, code: &str) -> String {
    view.pieces
        .iter()
        .find(|piece| piece.code == code)
        .unwrap_or_else(|| panic!("{code} is on the board"))
        .position
        .clone()
}

fn roll(session: &mut Session) {
    session
        .choose(Answer::Action("ROLL".to_string()))
        .expect("rolling is offered");
}

#[test]
fn the_view_is_a_map_with_two_tokens_and_one_action() {
    let session = Session::new_with_seed(GAME, 7).unwrap();
    let view = session.view();
    assert_eq!(view.nodes.len(), 101, "the start and the hundred squares");
    assert_eq!(token(&view, "RED_TOKEN"), "0");
    assert_eq!(token(&view, "BLUE_TOKEN"), "0");
    assert_eq!(view.turn, "RED");
    let actions: Vec<_> = view
        .choice
        .actions
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    assert_eq!(actions, ["ROLL"]);
    assert!(
        view.choice.places.is_empty(),
        "nothing to pick: the roll is the move"
    );
}

#[test]
fn a_roll_moves_the_token_and_the_dice_show_in_the_tray_not_the_seed() {
    let mut session = Session::new_with_seed(GAME, 7).unwrap();
    roll(&mut session);
    let view = session.view();
    assert_eq!(view.turn, "BLUE");
    assert_ne!(token(&view, "RED_TOKEN"), "0", "red has left the start");
    assert_eq!(token(&view, "BLUE_TOKEN"), "0");
    let shown = serde_json::to_string(&view).unwrap();
    assert!(
        !shown.contains("\"seed\"") && !shown.contains("draws"),
        "{shown}"
    );
}

#[test]
fn a_game_is_played_to_its_end_saved_and_resumed_after_every_roll() {
    for seed in [1_u64, 2, 3] {
        let mut straight = Session::new_with_seed(GAME, seed).unwrap();
        let mut stored = Session::new_with_seed(GAME, seed).unwrap();
        let mut turns = 0;
        while matches!(straight.view().status, Status::Playing) {
            roll(&mut straight);
            roll(&mut stored);
            let state = stored.state();
            stored = Session::resume(GAME, &state, turns + 1).unwrap();
            assert_eq!(straight.view(), stored.view(), "seed {seed}, turn {turns}");
            turns += 1;
            assert!(turns < 4000, "seed {seed} never ends");
        }
        match straight.view().status {
            Status::Won { winner } => {
                let code = format!("{winner}_TOKEN");
                assert_eq!(
                    token(&straight.view(), &code),
                    "100",
                    "the winner stands on 100"
                );
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
            roll(&mut session);
        }
        (
            token(&session.view(), "RED_TOKEN"),
            token(&session.view(), "BLUE_TOKEN"),
        )
    };
    assert_eq!(play(5), play(5));
    assert!((0..20).any(|seed| play(seed) != play(5)), "seeds differ");
}
