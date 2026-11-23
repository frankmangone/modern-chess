//! The hand and where removed pieces go: `hand` replaces `hand_enabled`, `removed_to` on an
//! effect sends the piece it removes to the mover's hand, and `captured_as` replaces
//! `demotes_to`.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Game, Piece};
    use crate::shared::Position;
    use crate::specs::{parse_game_spec_str, GameSpec, GameSpecError};
    use crate::tests::support::{go, pick};

    fn read(spec: &Value) -> Result<GameSpec, GameSpecError> {
        parse_game_spec_str(&spec.to_string())
    }

    /// A 4×4 game: WHITE's TAKER steps [0, 1] and takes what stands there. `removed_to` is
    /// written on the MOVE effect when given.
    fn spec(hand: Option<Value>, removed_to: Option<Value>) -> Value {
        let mut effect =
            json!({ "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } });
        if let Some(value) = removed_to {
            effect["removed_to"] = value;
        }
        let mut spec = json!({
            "name": "TAKES",
            "board": { "dimensions": [4, 4] },
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "pieces": [
                { "code": "TAKER", "name": "taker",
                  "moves": [{ "id": 0, "step": [0, 1], "do": [effect] }] },
                { "code": "PROMOTED", "name": "promoted", "captured_as": "TAKER", "moves": [] }
            ],
            "actions": [{ "name": "MOVE", "do": [
                { "choose": "PIECE", "where": { "owner": "ALLY" } },
                { "choose": "MOVE" }
            ] }]
        });
        if let Some(hand) = hand {
            spec["hand"] = hand;
        }
        spec
    }

    #[test]
    fn a_hand_is_switched_on_by_the_hand_key() {
        let on = spec(Some(json!({ "enabled": true })), None);
        assert!(read(&on).expect("reads").hand.enabled);
        assert!(!read(&spec(None, None)).expect("reads").hand.enabled);
        let off = spec(Some(json!({})), None);
        assert!(!read(&off).expect("reads").hand.enabled);
        let more = spec(Some(json!({ "enabled": true, "colour": "red" })), None);
        assert!(
            read(&more).is_err(),
            "a field the hand does not have is refused"
        );
    }

    #[test]
    fn the_old_hand_forms_are_refused_naming_their_replacement() {
        let mut old_switch = spec(None, None);
        old_switch["hand_enabled"] = json!(true);
        let mut old_table = spec(None, None);
        old_table["pieces"][1]["demotes_to"] = json!("TAKER");
        for (form, spec, replacement) in [
            (
                "hand_enabled",
                old_switch,
                "\"hand\": { \"enabled\": true }",
            ),
            ("demotes_to", old_table, "`captured_as`"),
        ] {
            let error = read(&spec)
                .err()
                .unwrap_or_else(|| panic!("{form} is refused"));
            assert!(
                matches!(error, GameSpecError::OldForm(_)),
                "{form}: {error}"
            );
            assert!(error.to_string().contains(replacement), "{form}: {error}");
        }
    }

    #[test]
    fn removed_to_needs_a_hand_and_a_known_destination() {
        let to_the_mover = json!({ "hand": "MOVER" });
        let enabled = json!({ "enabled": true });
        assert!(read(&spec(Some(enabled.clone()), Some(to_the_mover.clone()))).is_ok());

        let no_hand = read(&spec(None, Some(to_the_mover.clone()))).err();
        assert!(
            matches!(&no_hand, Some(GameSpecError::InvalidStep(why)) if why.contains("hand")),
            "{no_hand:?}"
        );
        let to_the_owner = read(&spec(
            Some(enabled.clone()),
            Some(json!({ "hand": "OWNER" })),
        ));
        assert!(
            to_the_owner
                .err()
                .is_some_and(|error| error.to_string().contains("4c-2")),
            "OWNER is refused, naming the step that adds it"
        );
        assert!(read(&spec(Some(enabled), Some(json!({ "place": "SUPPLY" })))).is_err());
    }

    #[test]
    fn captured_as_must_name_a_piece() {
        let mut unknown = spec(Some(json!({ "enabled": true })), None);
        unknown["pieces"][1]["captured_as"] = json!("NOBODY");
        assert!(matches!(
            read(&unknown),
            Err(GameSpecError::UnknownPieceInRule(_))
        ));
    }

    /// WHITE's TAKER at [1, 1] takes `victim` of `owner` at [1, 2] under `spec`; the code of
    /// every piece in WHITE's hand afterwards, with its count, sorted.
    fn hand_after_taking(spec: Value, victim: &str, owner: &str) -> Vec<(String, u32)> {
        let mut game = Game::from_spec(read(&spec).expect("reads"));
        game.state.pieces.clear();
        game.state
            .pieces
            .insert(pos![1, 1], Piece::new("TAKER", "WHITE"));
        game.state
            .pieces
            .insert(pos![1, 2], Piece::new(victim, owner));
        pick(&mut game, pos![1, 1]).expect("picks the taker");
        go(&mut game, pos![1, 2]).expect("takes");
        assert_eq!(
            game.state
                .pieces
                .get(&pos![1, 2])
                .map(|p| p.player.as_str()),
            Some("WHITE")
        );
        let mut held: Vec<(String, u32)> = game
            .hand()
            .get("WHITE")
            .into_iter()
            .flatten()
            .map(|(code, count)| (code.clone(), *count))
            .collect();
        held.sort();
        held
    }

    #[test]
    fn a_removed_enemy_piece_goes_to_the_hand_as_it_is_captured_as() {
        let enabled = json!({ "enabled": true });
        let sent = spec(Some(enabled), Some(json!({ "hand": "MOVER" })));
        assert_eq!(
            hand_after_taking(sent, "PROMOTED", "BLACK"),
            [("TAKER".to_string(), 1)]
        );
        let plain = spec(
            Some(json!({ "enabled": true })),
            Some(json!({ "hand": "MOVER" })),
        );
        assert_eq!(
            hand_after_taking(plain, "TAKER", "BLACK"),
            [("TAKER".to_string(), 1)]
        );
    }

    #[test]
    fn a_removal_without_removed_to_puts_nothing_in_the_hand() {
        let unsent = spec(Some(json!({ "enabled": true })), None);
        assert!(hand_after_taking(unsent, "PROMOTED", "BLACK").is_empty());
    }

    #[test]
    fn the_movers_own_removed_piece_never_enters_its_hand() {
        let sent = spec(
            Some(json!({ "enabled": true })),
            Some(json!({ "hand": "MOVER" })),
        );
        assert!(hand_after_taking(sent, "PROMOTED", "WHITE").is_empty());
    }
}
