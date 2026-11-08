//! What the domain allows once a match is being played: only an active match takes moves,
//! each one bumps the revision, and a finished game stays finished.
use chrono::{DateTime, Duration, TimeZone, Utc};
use modern_chess_game::{
    GameInstance, GameSpec, InstanceStatus, Outcome, PlayError, SeatName, Slug,
};
use modern_chess_kernel::{Handle, UserId};
use serde_json::json;

const CHESS: &str = include_str!("../../engine/specs/chess.json");

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap()
}

fn lobby() -> GameInstance {
    let spec = GameSpec::new(
        Slug::parse("chess").unwrap(),
        1,
        "Chess".to_string(),
        serde_json::from_str(CHESS).unwrap(),
        t0(),
    )
    .unwrap();
    GameInstance::new_lobby(&spec, t0()).unwrap()
}

fn seat(name: &str) -> SeatName {
    SeatName::new(name)
}

fn started() -> (GameInstance, UserId, UserId) {
    let (white, black) = (UserId::new(), UserId::new());
    let mut instance = lobby();
    instance
        .join(white, Handle::parse("Alice").unwrap(), seat("WHITE"), t0())
        .unwrap();
    instance
        .join(black, Handle::parse("Bobby").unwrap(), seat("BLACK"), t0())
        .unwrap();
    (instance, white, black)
}

#[test]
fn seat_of_names_the_seat_a_user_holds() {
    let (instance, white, black) = started();
    assert_eq!(instance.seat_of(white), Some(&seat("WHITE")));
    assert_eq!(instance.seat_of(black), Some(&seat("BLACK")));
    assert_eq!(instance.seat_of(UserId::new()), None);
}

#[test]
fn a_move_replaces_the_state_and_bumps_the_revision() {
    let (mut instance, _, _) = started();
    let before = instance.revision();

    instance
        .record_move(json!({ "after": 1 }), None, t0() + Duration::minutes(1))
        .unwrap();

    assert_eq!(instance.revision(), before + 1);
    assert_eq!(instance.state(), &json!({ "after": 1 }));
    assert_eq!(instance.status(), InstanceStatus::Active);
    assert_eq!(instance.outcome(), None);
}

#[test]
fn an_outcome_finishes_the_match() {
    let (mut instance, _, _) = started();
    let at = t0() + Duration::minutes(5);

    instance
        .record_move(json!({}), Some(Outcome::Draw), at)
        .unwrap();

    assert_eq!(instance.status(), InstanceStatus::Finished);
    assert_eq!(instance.outcome(), Some(&Outcome::Draw));
    assert_eq!(instance.finished_at(), Some(at));
}

#[test]
fn a_lobby_takes_no_moves_and_is_left_untouched() {
    let mut instance = lobby();
    let before = instance.clone();

    let result = instance.record_move(json!({ "x": 1 }), None, t0());

    assert_eq!(result, Err(PlayError::NotStarted));
    assert_eq!(instance, before);
}

#[test]
fn a_finished_match_takes_no_moves_and_is_left_untouched() {
    let (mut instance, _, _) = started();
    instance
        .record_move(json!({}), Some(Outcome::Draw), t0())
        .unwrap();
    let before = instance.clone();

    let result = instance.record_move(json!({ "x": 1 }), None, t0());

    assert_eq!(result, Err(PlayError::Over));
    assert_eq!(instance, before);
}
