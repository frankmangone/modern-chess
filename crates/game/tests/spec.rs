use chrono::{TimeZone, Utc};
use modern_chess_game::{DomainError, GameSpec, SeatName, Slug};
use serde_json::{json, Value};

const CHESS: &str = include_str!("../../engine/specs/chess.json");
const FOUR_PLAYER: &str = include_str!("../../engine/specs/4player_chess.json");

fn spec_from(slug: &str, rules: Value) -> Result<GameSpec, DomainError> {
    let now = Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap();
    GameSpec::new(
        Slug::parse(slug).unwrap(),
        1,
        "Name".to_string(),
        rules,
        now,
    )
}

fn seats(rules: &str) -> Vec<SeatName> {
    let spec = spec_from("any", serde_json::from_str(rules).unwrap()).unwrap();
    spec.seats().unwrap()
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

#[test]
fn spec_reports_two_seats_for_chess() {
    let names: Vec<_> = seats(CHESS).iter().map(|s| s.to_string()).collect();
    assert_eq!(names, ["WHITE", "BLACK"]);
}

#[test]
fn spec_reports_four_seats_for_4player_chess() {
    let names: Vec<_> = seats(FOUR_PLAYER).iter().map(|s| s.to_string()).collect();
    assert_eq!(names, ["WHITE", "BLACK", "SILVER", "GOLD"]);
}

#[test]
fn spec_rejects_rules_the_engine_cannot_parse() {
    let result = spec_from("broken", json!({ "name": "not a game" }));
    assert!(matches!(result, Err(DomainError::InvalidRules(_))));
}

#[test]
fn spec_hash_is_stable_for_identical_rules_and_changes_with_them() {
    let rules: Value = serde_json::from_str(CHESS).unwrap();
    let a = spec_from("chess", rules.clone()).unwrap();
    let b = spec_from("chess", rules.clone()).unwrap();
    assert_eq!(a.rules_hash, b.rules_hash);
    assert_eq!(a.rules_hash.len(), 64);

    let mut changed = rules;
    changed["name"] = json!("Chess, revised");
    let c = spec_from("chess", changed).unwrap();
    assert_ne!(a.rules_hash, c.rules_hash);
}
