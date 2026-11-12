#[cfg(test)]
mod tests {
    use crate::specs::parse_game_spec_str;

    const CHATURANGA: &str = include_str!("../../../specs/chaturanga.json");

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
}
