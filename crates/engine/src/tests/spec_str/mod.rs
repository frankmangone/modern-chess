#[cfg(test)]
mod tests {
    use crate::specs::{parse_game_spec, parse_game_spec_str, GameSpecError};
    use serde_json::{json, Value};

    const CHATURANGA: &str = include_str!("../../../specs/chaturanga.json");
    const JANGGI_CANNON: &str = include_str!("../../../specs/janggi_cannon.json");

    #[test]
    fn parses_a_valid_spec_from_text() {
        let spec = parse_game_spec_str(CHATURANGA).expect("chaturanga should parse");
        assert!(!spec.name.is_empty());
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(parse_game_spec_str("{").is_err());
    }

    #[test]
    fn rejects_unknown_piece_in_starting_position() {
        let broken = CHATURANGA.replacen("\"piece\": \"PADATI\"", "\"piece\": \"NOPE\"", 1);
        assert_ne!(broken, CHATURANGA, "fixture must actually change the text");
        assert!(parse_game_spec_str(&broken).is_err());
    }

    #[test]
    fn rejects_a_spec_with_nobody_to_move() {
        let mut spec: serde_json::Value = serde_json::from_str(CHATURANGA).unwrap();
        spec["turns"]["order"] = serde_json::json!([]);
        assert!(parse_game_spec_str(&spec.to_string()).is_err());
    }

    /// A repeating move whose step goes nowhere never leaves its square: it used to count past
    /// the end of its counter (a panic in debug builds, a hang in release builds).
    #[test]
    fn rejects_a_move_that_goes_nowhere() {
        for step in [serde_json::json!([]), serde_json::json!([0, 0])] {
            let mut spec: serde_json::Value = serde_json::from_str(JANGGI_CANNON).unwrap();
            spec["pieces"][2]["moves"][4]["step"] = step.clone();
            assert!(
                parse_game_spec_str(&spec.to_string()).is_err(),
                "step {step} should be refused"
            );
        }
    }

    fn chaturanga() -> Value {
        serde_json::from_str(CHATURANGA).unwrap()
    }

    fn check(spec: &Value) -> Result<(), GameSpecError> {
        parse_game_spec_str(&spec.to_string()).map(|_| ())
    }

    #[test]
    fn accepts_every_shipped_spec() {
        for entry in std::fs::read_dir("./specs").unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            assert!(
                parse_game_spec(&path).is_ok(),
                "{} should parse",
                path.display()
            );
        }
    }

    #[test]
    fn rejects_a_direction_of_the_wrong_size() {
        let mut spec = chaturanga();
        spec["players"][0]["direction"] = json!([[1, 0, 0], [0, 1, 0], [0, 0, 1]]);
        assert!(matches!(
            check(&spec),
            Err(GameSpecError::MismatchedDimensions {
                expected: 2,
                found: 3,
                ..
            })
        ));
    }

    #[test]
    fn rejects_a_direction_that_is_not_a_rotation() {
        let mut spec = chaturanga();
        spec["players"][0]["direction"] = json!([[2, 0], [0, 1]]);
        assert!(matches!(
            check(&spec),
            Err(GameSpecError::InvalidDirectionMatrix(_))
        ));
    }

    #[test]
    fn rejects_a_step_of_the_wrong_size() {
        let mut spec = chaturanga();
        spec["pieces"][0]["moves"][0]["step"] = json!([1, 0, 0]);
        assert!(matches!(
            check(&spec),
            Err(GameSpecError::MismatchedDimensions {
                expected: 2,
                found: 3,
                ..
            })
        ));
    }

    #[test]
    fn rejects_an_offset_of_the_wrong_size() {
        let mut in_a_condition = chaturanga();
        in_a_condition["pieces"][0]["moves"][0]["conditions"] = json!([{
            "test": "COUNT",
            "in": { "region": "AT", "at": { "place": "START", "offset": [1, 0, 0] } }
        }]);
        let mut in_a_move_side_effect = chaturanga();
        in_a_move_side_effect["pieces"][0]["moves"][0]["side_effects"] =
            json!([{"action": "CAPTURE", "target": [0, 1, 0]}]);
        let mut in_an_action_side_effect = chaturanga();
        in_an_action_side_effect["pieces"][0]["moves"][0]["actions"][0]["side_effects"] =
            json!([{"action": "MOVE", "from": [0, 0], "to": [1]}]);

        for spec in [
            in_a_condition,
            in_a_move_side_effect,
            in_an_action_side_effect,
        ] {
            assert!(matches!(
                check(&spec),
                Err(GameSpecError::MismatchedDimensions { .. })
            ));
        }
    }

    #[test]
    fn rejects_boards_that_are_empty_or_too_large() {
        for dimensions in [
            json!([]),
            json!([8, 0]),
            json!([101, 100]),
            json!([1, 1, 1, 1, 1, 1, 1, 1, 1]),
        ] {
            let mut spec = chaturanga();
            spec["board"]["dimensions"] = dimensions.clone();
            assert!(
                matches!(check(&spec), Err(GameSpecError::InvalidBoardSize(_))),
                "{dimensions} should be refused"
            );
        }
    }

    #[test]
    fn accepts_a_board_of_exactly_ten_thousand_positions() {
        let mut spec = chaturanga();
        spec["board"]["dimensions"] = json!([100, 100]);
        assert!(check(&spec).is_ok());
    }

    /// No board is wider than 255, so a larger step or offset always lands off the board; it
    /// would also overflow when added to a square.
    #[test]
    fn rejects_steps_and_offsets_longer_than_any_board() {
        let mut far_condition = chaturanga();
        far_condition["pieces"][0]["moves"][0]["conditions"] = json!([{
            "test": "COUNT",
            "in": { "region": "AT", "at": { "place": "START", "offset": [32767, 0] } }
        }]);
        let mut far_step = chaturanga();
        far_step["pieces"][0]["moves"][0]["step"] = json!([256, 0]);
        let mut far_target = chaturanga();
        far_target["pieces"][0]["moves"][0]["side_effects"] =
            json!([{"action": "CAPTURE", "target": [-256, 0]}]);

        let mut most_negative = chaturanga();
        most_negative["pieces"][0]["moves"][0]["step"] = json!([i16::MIN, 0]);

        for spec in [far_condition, far_step, far_target, most_negative] {
            assert!(matches!(
                check(&spec),
                Err(GameSpecError::ValueOutOfRange { .. })
            ));
        }
    }

    #[test]
    fn accepts_a_step_as_long_as_the_widest_board() {
        let mut spec = chaturanga();
        spec["pieces"][0]["moves"][0]["step"] = json!([255, 0]);
        assert!(check(&spec).is_ok());
    }

    /// A rotation of a grid only ever swaps axes and flips them; larger entries (a shear such as
    /// `[[1, 32767], [0, 1]]` still has determinant 1) would overflow turning a step.
    #[test]
    fn rejects_a_direction_with_entries_other_than_minus_one_zero_or_one() {
        let mut spec = chaturanga();
        spec["players"][0]["direction"] = json!([[1, 32767], [0, 1]]);
        assert!(matches!(
            check(&spec),
            Err(GameSpecError::InvalidDirectionMatrix(_))
        ));
    }
}
