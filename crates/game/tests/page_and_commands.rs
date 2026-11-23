use modern_chess_game::{CommandKind, GamePage, Outcome, PageSection, SeatName, WinReason};
use serde_json::json;

#[test]
fn game_page_defaults_missing_optional_fields() {
    let page: GamePage = serde_json::from_value(json!({
        "slug": "chess",
        "tagline": "The classic",
        "description": "Kings, queens, and a long history."
    }))
    .unwrap();

    assert!(page.tags.is_empty());
    assert_eq!(page.accent, None);
    assert_eq!(page.cover_image, None);
    assert_eq!(page.skin, None);
    assert!(page.sections.is_empty());
    assert!(!page.featured);
    assert_eq!(page.sort_order, 0);
}

#[test]
fn game_page_keeps_section_order() {
    let value = json!({
        "slug": "chess",
        "tagline": "t",
        "description": "d",
        "sections": [
            { "type": "how_to_play", "markdown": "Move pieces." },
            { "type": "preview" },
            { "type": "open_lobbies" }
        ]
    });

    let page: GamePage = serde_json::from_value(value).unwrap();

    assert_eq!(
        page.sections,
        vec![
            PageSection::HowToPlay {
                markdown: "Move pieces.".to_string()
            },
            PageSection::Preview,
            PageSection::OpenLobbies,
        ]
    );
    let again = serde_json::to_value(&page).unwrap();
    assert_eq!(again["sections"][0]["type"], "how_to_play");
    assert_eq!(again["sections"][2]["type"], "open_lobbies");
}

#[test]
fn game_page_rejects_an_invalid_slug() {
    let result = serde_json::from_value::<GamePage>(json!({
        "slug": "Not A Slug",
        "tagline": "t",
        "description": "d"
    }));
    assert!(result.is_err());
}

fn choose(
    place: Option<&str>,
    code: Option<&str>,
    held: Option<&str>,
    action: Option<&str>,
) -> CommandKind {
    let own = |text: Option<&str>| text.map(str::to_string);
    CommandKind::Choose {
        place: own(place),
        code: own(code),
        held: own(held),
        action: own(action),
    }
}

#[test]
fn command_kind_serialises_with_a_type_tag() {
    assert_eq!(
        serde_json::to_value(choose(Some("4,3"), None, None, None)).unwrap(),
        json!({ "type": "choose", "place": "4,3" })
    );
    assert_eq!(
        serde_json::to_value(choose(None, Some("QUEEN"), None, None)).unwrap(),
        json!({ "type": "choose", "code": "QUEEN" })
    );
    assert_eq!(
        serde_json::to_value(choose(None, None, Some("PAWN"), None)).unwrap(),
        json!({ "type": "choose", "held": "PAWN" })
    );
    assert_eq!(
        serde_json::to_value(choose(None, None, None, Some("PASS"))).unwrap(),
        json!({ "type": "choose", "action": "PASS" })
    );
    assert_eq!(
        serde_json::to_value(CommandKind::Cancel).unwrap(),
        json!({ "type": "cancel" })
    );
}

#[test]
fn a_logged_drop_reads_as_a_hand_piece_and_a_place_but_is_never_accepted() {
    let logged: CommandKind =
        serde_json::from_value(json!({ "type": "choose", "hand": "PAWN", "place": "2,2" }))
            .unwrap();

    assert_eq!(logged, choose(Some("2,2"), None, Some("PAWN"), None));
    assert!(logged.refusal().is_some());
}

#[test]
fn outcome_roundtrips_through_json() {
    for outcome in [
        Outcome::Won {
            seat: SeatName::new("WHITE"),
            reason: None,
        },
        Outcome::Won {
            seat: SeatName::new("WHITE"),
            reason: Some(WinReason::Resigned),
        },
        Outcome::Draw,
    ] {
        let json = serde_json::to_string(&outcome).unwrap();
        assert_eq!(serde_json::from_str::<Outcome>(&json).unwrap(), outcome);
    }
}

#[test]
fn a_win_by_play_has_no_reason_in_its_json_and_stored_wins_still_load() {
    let by_play = Outcome::Won {
        seat: SeatName::new("WHITE"),
        reason: None,
    };
    assert_eq!(
        serde_json::to_value(&by_play).unwrap(),
        json!({ "type": "won", "seat": "WHITE" })
    );
    assert_eq!(
        serde_json::from_value::<Outcome>(json!({ "type": "won", "seat": "WHITE" })).unwrap(),
        by_play
    );
    assert_eq!(
        serde_json::to_value(Outcome::Won {
            seat: SeatName::new("WHITE"),
            reason: Some(WinReason::Resigned),
        })
        .unwrap(),
        json!({ "type": "won", "seat": "WHITE", "reason": "resigned" })
    );
}

#[test]
fn resign_serialises_as_a_bare_tagged_command() {
    assert_eq!(
        serde_json::to_value(CommandKind::Resign).unwrap(),
        json!({ "type": "resign" })
    );
}
