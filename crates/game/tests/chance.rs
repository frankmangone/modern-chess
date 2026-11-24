//! A new match gets a seed for its chance (dice, shuffles), kept in its stored state. A game that
//! never rolls or shuffles gets none, so its state is what it always was.

use chrono::{TimeZone, Utc};
use modern_chess_game::{GameInstance, GameSpec, Slug};
use serde_json::Value;

const RACE: &str = include_str!("../../session/tests/fixtures/dice_race.json");
const CHESS: &str = include_str!("../../engine/specs/chess.json");

fn lobby(slug: &str, rules: &str) -> GameInstance {
    let now = Utc.with_ymd_and_hms(2026, 11, 27, 12, 0, 0).unwrap();
    let spec = GameSpec::new(
        Slug::parse(slug).unwrap(),
        1,
        "Name".to_string(),
        serde_json::from_str::<Value>(rules).unwrap(),
        now,
    )
    .unwrap();
    GameInstance::new_lobby(&spec, now).unwrap()
}

#[test]
fn a_match_that_rolls_is_born_with_a_seed_of_its_own() {
    let first = lobby("race", RACE);
    let second = lobby("race", RACE);
    let seed = |instance: &GameInstance| instance.state()["seed"].as_u64();
    assert!(seed(&first).is_some(), "{}", first.state());
    assert!(seed(&second).is_some());
    assert_ne!(seed(&first), seed(&second), "two matches share no seed");
    assert!(first.state().get("draws").is_none(), "nothing is drawn yet");
}

#[test]
fn a_match_that_never_rolls_has_no_seed_in_its_state() {
    let chess = lobby("chess", CHESS);
    assert!(chess.state().get("seed").is_none());
    assert!(chess.state().get("draws").is_none());
}
