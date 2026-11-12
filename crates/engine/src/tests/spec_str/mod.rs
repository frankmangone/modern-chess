#[cfg(test)]
mod tests {
    use crate::specs::parse_game_spec_str;

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
}
