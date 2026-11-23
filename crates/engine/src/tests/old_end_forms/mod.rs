//! The keys that ended a game before `end_checks` are refused, with a message naming what
//! replaced them.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::specs::{parse_game_spec_str, GameSpecError};

    /// Chess with the old key written in.
    fn refused_with(key: &str, value: Value) -> String {
        let mut spec: Value = serde_json::from_str(
            &std::fs::read_to_string("./specs/chess.json").expect("the spec reads"),
        )
        .expect("the spec is JSON");
        spec[key] = value;
        let error = parse_game_spec_str(&spec.to_string()).expect_err("the old key is refused");
        assert!(matches!(error, GameSpecError::OldForm(_)), "{error:?}");
        error.to_string()
    }

    #[test]
    fn win_conditions_is_refused_with_old_form() {
        let message = refused_with(
            "win_conditions",
            json!([{ "type": "PIECE_IN_ZONE", "piece": "KING", "zone": "HILL" }]),
        );
        assert!(message.contains("end_checks"), "{message}");
        assert!(message.contains("WIN"), "{message}");
    }

    #[test]
    fn draw_conditions_is_refused_with_old_form() {
        let message = refused_with("draw_conditions", json!({ "repetition_count": 3 }));
        assert!(message.contains("end_checks"), "{message}");
        assert!(message.contains("DRAW"), "{message}");
    }

    #[test]
    fn leader_is_refused_with_old_form() {
        let message = refused_with("leader", json!(["KING"]));
        assert!(message.contains("end_checks"), "{message}");
        assert!(message.contains("STUCK"), "{message}");
    }

    #[test]
    fn stalemate_loses_is_refused_with_old_form() {
        let message = refused_with("stalemate_loses", json!(true));
        assert!(message.contains("end_checks"), "{message}");
        assert!(message.contains("STUCK"), "{message}");
    }
}
