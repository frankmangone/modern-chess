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

#[test]
fn command_kind_serialises_with_a_type_tag() {
    assert_eq!(
        serde_json::to_value(CommandKind::Move {
            position: "4,3".to_string()
        })
        .unwrap(),
        json!({ "type": "move", "position": "4,3" })
    );
    assert_eq!(
        serde_json::to_value(CommandKind::Select {
            position: "1,0".to_string()
        })
        .unwrap(),
        json!({ "type": "select", "position": "1,0" })
    );
    assert_eq!(
        serde_json::to_value(CommandKind::Promote {
            code: "QUEEN".to_string()
        })
        .unwrap(),
        json!({ "type": "promote", "code": "QUEEN" })
    );
    assert_eq!(
        serde_json::to_value(CommandKind::Drop {
            code: "PAWN".to_string(),
            position: "2,2".to_string()
        })
        .unwrap(),
        json!({ "type": "drop", "code": "PAWN", "position": "2,2" })
    );
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
