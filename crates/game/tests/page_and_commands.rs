use modern_chess_game::{CommandKind, GamePage, Outcome, PageSection, SeatName};
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
        serde_json::to_value(CommandKind::Move { x: 4, y: 3 }).unwrap(),
        json!({ "type": "move", "x": 4, "y": 3 })
    );
    assert_eq!(
        serde_json::to_value(CommandKind::Select { x: 1, y: 0 }).unwrap(),
        json!({ "type": "select", "x": 1, "y": 0 })
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
            x: 2,
            y: 2
        })
        .unwrap(),
        json!({ "type": "drop", "code": "PAWN", "x": 2, "y": 2 })
    );
}

#[test]
fn outcome_roundtrips_through_json() {
    for outcome in [
        Outcome::Won {
            seat: SeatName::new("WHITE"),
        },
        Outcome::Draw,
    ] {
        let json = serde_json::to_string(&outcome).unwrap();
        assert_eq!(serde_json::from_str::<Outcome>(&json).unwrap(), outcome);
    }
}
