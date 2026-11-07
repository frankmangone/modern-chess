use chrono::{DateTime, Duration, TimeZone, Utc};
use modern_chess_game::{GameInstance, GameSpec, InstanceStatus, LobbyError, SeatName, Slug};
use modern_chess_kernel::{Handle, UserId};

const CHESS: &str = include_str!("../../engine/specs/chess.json");
const FOUR_PLAYER: &str = include_str!("../../engine/specs/4player_chess.json");

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 7, 12, 0, 0).unwrap()
}

fn spec(slug: &str, rules: &str) -> GameSpec {
    GameSpec::new(
        Slug::parse(slug).unwrap(),
        3,
        "Name".to_string(),
        serde_json::from_str(rules).unwrap(),
        t0(),
    )
    .unwrap()
}

fn chess_lobby() -> GameInstance {
    GameInstance::new_lobby(&spec("chess", CHESS), t0()).unwrap()
}

fn player(name: &str) -> (UserId, Handle) {
    (UserId::new(), Handle::parse(name).unwrap())
}

fn seat(name: &str) -> SeatName {
    SeatName::new(name)
}

#[test]
fn new_lobby_is_empty_with_capacity_from_the_spec() {
    let lobby = chess_lobby();
    assert_eq!(lobby.status(), InstanceStatus::Lobby);
    assert!(lobby.players().is_empty());
    assert_eq!(lobby.capacity(), 2);
    assert_eq!(lobby.slug().as_str(), "chess");
    assert_eq!(lobby.spec_version(), 3);
    assert_eq!(lobby.revision(), 0);
    assert!(lobby.started_at().is_none());
    assert!(
        lobby.state().is_object(),
        "starts from the engine's opening position"
    );
}

#[test]
fn join_takes_the_requested_seat_and_keeps_the_handle() {
    let mut lobby = chess_lobby();
    let (user, handle) = player("Alice");

    lobby
        .join(user, handle.clone(), seat("BLACK"), t0())
        .unwrap();

    let joined = &lobby.players()[0];
    assert_eq!(joined.seat, seat("BLACK"));
    assert_eq!(joined.user, user);
    assert_eq!(joined.handle, handle);
    assert_eq!(joined.joined_at, t0());
    assert_eq!(lobby.status(), InstanceStatus::Lobby);
    assert!(!lobby.is_full());
}

#[test]
fn join_rejects_a_taken_seat() {
    let mut lobby = chess_lobby();
    let (a, ha) = player("Alice");
    let (b, hb) = player("Bobby");
    lobby.join(a, ha, seat("WHITE"), t0()).unwrap();

    let err = lobby.join(b, hb, seat("WHITE"), t0()).unwrap_err();

    assert_eq!(err, LobbyError::SeatTaken);
    assert_eq!(lobby.players().len(), 1);
}

#[test]
fn join_rejects_a_second_seat_for_the_same_user() {
    let mut lobby = chess_lobby();
    let (a, ha) = player("Alice");
    lobby.join(a, ha.clone(), seat("WHITE"), t0()).unwrap();

    let err = lobby.join(a, ha, seat("BLACK"), t0()).unwrap_err();

    assert_eq!(err, LobbyError::AlreadySeated);
    assert_eq!(lobby.players().len(), 1);
}

#[test]
fn join_rejects_an_unknown_seat() {
    let mut lobby = chess_lobby();
    let (a, ha) = player("Alice");

    let err = lobby.join(a, ha, seat("PURPLE"), t0()).unwrap_err();

    assert_eq!(err, LobbyError::UnknownSeat);
    assert!(lobby.players().is_empty());
}

#[test]
fn last_join_makes_the_instance_active_and_sets_started_at() {
    let mut lobby = chess_lobby();
    let (a, ha) = player("Alice");
    let (b, hb) = player("Bobby");
    let later = t0() + Duration::minutes(5);

    lobby.join(a, ha, seat("WHITE"), t0()).unwrap();
    lobby.join(b, hb, seat("BLACK"), later).unwrap();

    assert!(lobby.is_full());
    assert_eq!(lobby.status(), InstanceStatus::Active);
    assert_eq!(lobby.started_at(), Some(later));
}

#[test]
fn four_seat_lobby_stays_a_lobby_until_the_fourth_join() {
    let mut lobby = GameInstance::new_lobby(&spec("4player-chess", FOUR_PLAYER), t0()).unwrap();
    for (i, name) in ["WHITE", "BLACK", "SILVER"].into_iter().enumerate() {
        let (u, h) = player(&format!("Player{i}"));
        lobby.join(u, h, seat(name), t0()).unwrap();
        assert_eq!(
            lobby.status(),
            InstanceStatus::Lobby,
            "after {} joins",
            i + 1
        );
    }
    let (u, h) = player("Player3");
    lobby.join(u, h, seat("GOLD"), t0()).unwrap();
    assert_eq!(lobby.status(), InstanceStatus::Active);
}

#[test]
fn join_rejects_when_not_in_lobby() {
    let mut lobby = chess_lobby();
    let (a, ha) = player("Alice");
    let (b, hb) = player("Bobby");
    let (c, hc) = player("Carol");
    lobby.join(a, ha, seat("WHITE"), t0()).unwrap();
    lobby.join(b, hb, seat("BLACK"), t0()).unwrap();

    let err = lobby.join(c, hc, seat("WHITE"), t0()).unwrap_err();

    assert_eq!(err, LobbyError::NotInLobby);
}

#[test]
fn first_free_seat_follows_the_specs_seat_order() {
    let mut lobby = chess_lobby();
    assert_eq!(lobby.first_free_seat(), Some(&seat("WHITE")));

    let (a, ha) = player("Alice");
    lobby.join(a, ha, seat("WHITE"), t0()).unwrap();
    assert_eq!(lobby.first_free_seat(), Some(&seat("BLACK")));

    let (b, hb) = player("Bobby");
    lobby.join(b, hb, seat("BLACK"), t0()).unwrap();
    assert_eq!(lobby.first_free_seat(), None);
}

#[test]
fn status_moves_only_lobby_to_active_to_finished() {
    use InstanceStatus::{Active, Finished, Lobby};
    assert!(Lobby.can_become(Active));
    assert!(Active.can_become(Finished));

    assert!(!Lobby.can_become(Lobby));
    assert!(!Lobby.can_become(Finished));
    assert!(!Active.can_become(Lobby));
    assert!(!Active.can_become(Active));
}

#[test]
fn finished_is_terminal() {
    use InstanceStatus::{Active, Finished, Lobby};
    for next in [Lobby, Active, Finished] {
        assert!(
            !Finished.can_become(next),
            "Finished must not become {next:?}"
        );
    }
}

#[test]
fn restore_roundtrips_a_valid_instance() {
    let mut lobby = chess_lobby();
    let (a, ha) = player("Alice");
    lobby.join(a, ha, seat("WHITE"), t0()).unwrap();

    let restored = GameInstance::restore(lobby.to_parts()).unwrap();

    assert_eq!(restored.players(), lobby.players());
    assert_eq!(restored.status(), InstanceStatus::Lobby);
}

#[test]
fn restore_rejects_a_player_in_an_unknown_seat() {
    let mut lobby = chess_lobby();
    let (a, ha) = player("Alice");
    lobby.join(a, ha, seat("WHITE"), t0()).unwrap();
    let mut parts = lobby.to_parts();
    parts.players[0].seat = seat("PURPLE");

    assert!(GameInstance::restore(parts).is_err());
}

#[test]
fn restore_rejects_an_active_instance_with_empty_seats() {
    let mut parts = chess_lobby().to_parts();
    parts.status = InstanceStatus::Active;
    parts.started_at = Some(t0());

    assert!(GameInstance::restore(parts).is_err());
}
