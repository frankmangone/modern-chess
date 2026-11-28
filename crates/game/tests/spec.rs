use chrono::{TimeZone, Utc};
use modern_chess_game::{Engine, EngineError, GameSpec, SeatName, Slug};
use serde_json::{json, Value};

const CHESS: &str = include_str!("../../engine/specs/chess.json");
const FOUR_PLAYER: &str = include_str!("../../engine/specs/4player_chess.json");

fn spec_from(slug: &str, rules: Value) -> GameSpec {
    let now = Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap();
    GameSpec::new(
        Slug::parse(slug).unwrap(),
        1,
        "Name".to_string(),
        rules,
        now,
    )
}

/// The seats, asked of the engine in its sandbox: the way the server gets them.
async fn seats(rules: &str) -> Vec<SeatName> {
    let spec = spec_from("any", serde_json::from_str(rules).unwrap());
    Engine::new().seats(&spec).await.unwrap()
}

#[test]
fn slug_rejects_uppercase_spaces_and_empty() {
    for bad in ["", "Chess", "two words", "under_score", &"a".repeat(65)] {
        assert!(Slug::parse(bad).is_err(), "{bad:?} should be rejected");
    }
}

#[test]
fn slug_accepts_lowercase_digits_and_dashes() {
    for good in ["chess", "4player-chess", "a", "a-b-c", &"a".repeat(64)] {
        assert_eq!(Slug::parse(good).unwrap().as_str(), good);
    }
}

#[tokio::test]
async fn spec_reports_two_seats_for_chess() {
    let names: Vec<_> = seats(CHESS).await.iter().map(|s| s.to_string()).collect();
    assert_eq!(names, ["WHITE", "BLACK"]);
}

#[tokio::test]
async fn spec_reports_four_seats_for_4player_chess() {
    let names: Vec<_> = seats(FOUR_PLAYER)
        .await
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(names, ["WHITE", "BLACK", "SILVER", "GOLD"]);
}

#[tokio::test]
async fn the_engine_refuses_rules_it_cannot_parse() {
    let result = Engine::new().check(&json!({ "name": "not a game" })).await;
    assert!(
        matches!(result, Err(EngineError::Unusable(_))),
        "{result:?}"
    );
}

#[tokio::test]
async fn the_engine_says_what_good_rules_are() {
    let rules: Value = serde_json::from_str(CHESS).unwrap();
    let summary = Engine::new().check(&rules).await.unwrap();
    assert_eq!(summary.name, "CHESS");
    assert_eq!(summary.players.len(), 2);
}

#[tokio::test]
async fn an_engine_with_no_time_to_spend_stops_and_says_so() {
    let starved = Engine::with_limits(modern_chess_sandbox::Limits {
        fuel: 10_000,
        ..Default::default()
    });
    let rules: Value = serde_json::from_str(CHESS).unwrap();
    assert!(matches!(
        starved.check(&rules).await,
        Err(EngineError::Stopped(_))
    ));
}

#[test]
fn spec_hash_is_stable_for_identical_rules_and_changes_with_them() {
    let rules: Value = serde_json::from_str(CHESS).unwrap();
    let a = spec_from("chess", rules.clone());
    let b = spec_from("chess", rules.clone());
    assert_eq!(a.rules_hash, b.rules_hash);
    assert_eq!(a.rules_hash.len(), 64);

    let mut changed = rules;
    changed["name"] = json!("Chess, revised");
    let c = spec_from("chess", changed);
    assert_ne!(a.rules_hash, c.rules_hash);
}
