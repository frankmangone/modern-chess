//! Fox and Geese through the session: what each viewer is shown, a goose taken by a jump, a jump
//! that goes on, and whole games of random play that always end, saved and resumed on the way.

mod support;

use modern_chess_session::{Session, Status, Viewer};
use support::{at, destinations, Dice};

const FOX_AND_GEESE: &str = include_str!("../../engine/specs/fox_and_geese.json");

fn session() -> Session {
    Session::new(FOX_AND_GEESE).expect("fox and geese reads")
}

fn count(session: &Session, player: &str) -> usize {
    session
        .view()
        .pieces
        .iter()
        .filter(|piece| piece.player == player)
        .count()
}

#[test]
fn the_geese_move_first_and_only_the_geese_are_offered_anything() {
    let session = session();
    let geese = session.view_for(&Viewer::Player("GEESE".into()));
    let fox = session.view_for(&Viewer::Player("FOX".into()));
    assert_eq!(geese.turn, "GEESE");
    assert!(!geese.choice.places.is_empty());
    assert!(fox.choice.places.is_empty(), "the fox waits");
    assert_eq!(count(&session, "GEESE"), 13);
    assert_eq!(count(&session, "FOX"), 1);
    assert_eq!(geese.nodes.len(), 33, "the cross has 33 points");
}

#[test]
fn a_goose_is_offered_only_forward_and_sideways() {
    let mut session = session();
    let pieces: Vec<_> = session
        .view()
        .choice
        .places
        .iter()
        .map(|place| place.name.as_str().to_string())
        .collect();
    assert!(pieces.contains(&"0,2".to_string()));
    assert!(
        !pieces.contains(&"3,2".to_string()),
        "the goose in front of the fox has nowhere to go"
    );
    at(&mut session, "0,2").unwrap();
    assert_eq!(destinations(&session.view()), ["0,3"]);
}

/// One answer from what is offered: the piece to move, or where to.
fn answer_at_random(session: &mut Session, dice: &mut Dice) {
    let offered: Vec<_> = session
        .view()
        .choice
        .places
        .iter()
        .map(|place| place.name.as_str().to_string())
        .collect();
    assert!(
        !offered.is_empty(),
        "a game that is playing offers an answer"
    );
    at(session, &offered[dice.below(offered.len())]).unwrap();
}

/// Whole games of random play, always from what the board offers. Every game ends (by a trapped
/// fox, too few geese, geese with no move or a repeated position), and after every answer, a jump
/// half done included, the saved game resumes as the same game.
#[test]
fn random_games_end_and_resume_at_every_step() {
    let (mut geese_won, mut fox_won, mut drawn) = (0, 0, 0);
    for seed in 0..12u64 {
        let mut dice = Dice(seed + 11);
        let mut game = session();
        let mut geese = 13;
        let mut answers = 0;
        while matches!(game.view().status, Status::Playing) {
            answers += 1;
            assert!(answers < 20_000, "seed {seed} does not end");
            answer_at_random(&mut game, &mut dice);
            let resumed = Session::resume(FOX_AND_GEESE, &game.state(), game.revision())
                .unwrap_or_else(|error| panic!("seed {seed}, answer {answers}: {error}"));
            assert_eq!(
                resumed.state(),
                game.state(),
                "seed {seed}, answer {answers}"
            );
            assert_eq!(resumed.view(), game.view(), "seed {seed}, answer {answers}");
            let now = count(&game, "GEESE");
            assert!(now <= geese, "a goose never comes back");
            geese = now;
            assert_eq!(count(&game, "FOX"), 1, "seed {seed}");
        }
        match &game.view().status {
            Status::Won { winner } if winner == "GEESE" => geese_won += 1,
            Status::Won { .. } => fox_won += 1,
            Status::Playing => unreachable!("the loop ended"),
            Status::Draw => drawn += 1,
        }
    }
    assert_eq!(geese_won + fox_won + drawn, 12);
    assert!(
        fox_won > 0,
        "random geese walk into a jammed corner or lose their numbers"
    );
}
