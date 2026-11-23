//! A key a spec writes that the format does not have is refused, wherever it is written.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::specs::parse_game_spec_str;

    fn load(path: &str) -> Value {
        serde_json::from_str(&std::fs::read_to_string(path).expect("the spec reads"))
            .expect("the spec is JSON")
    }

    /// Asserts `spec` is refused and the message names `key`.
    pub(super) fn refused_naming(spec: &Value, key: &str) {
        let error = parse_game_spec_str(&spec.to_string())
            .err()
            .unwrap_or_else(|| panic!("the spec naming {key} should be refused"))
            .to_string();
        assert!(error.contains(key), "the refusal names {key}: {error}");
    }

    fn rook(spec: &Value) -> usize {
        spec["pieces"]
            .as_array()
            .expect("pieces")
            .iter()
            .position(|piece| piece["code"] == "ROOK")
            .expect("a rook")
    }

    #[test]
    fn a_bogus_key_is_refused_at_every_level() {
        let chess = load("./specs/chess.json");
        let rook_repeat = format!("/pieces/{}/moves/0/repeat", rook(&chess));
        let kalah = load("./src/tests/kalah/spec.json");
        let levels: Vec<(&str, &Value, String)> = vec![
            ("the top level", &chess, String::new()),
            ("board", &chess, "/board".into()),
            ("a player", &chess, "/players/0".into()),
            (
                "a starting position",
                &chess,
                "/players/0/starting_positions/0".into(),
            ),
            ("a piece", &chess, "/pieces/0".into()),
            ("a move", &chess, "/pieces/0/moves/0".into()),
            ("a repeat", &chess, rook_repeat),
            ("turns", &chess, "/turns".into()),
            ("a map place", &kalah, "/board/places/0".into()),
        ];
        for (level, spec, pointer) in levels {
            assert!(
                parse_game_spec_str(&spec.to_string()).is_ok(),
                "{level}: the spec reads before the key is added"
            );
            let mut spec = spec.clone();
            spec.pointer_mut(&pointer)
                .unwrap_or_else(|| panic!("{level}: no object at {pointer}"))
                .as_object_mut()
                .unwrap_or_else(|| panic!("{level}: {pointer} is not an object"))
                .insert("bogus".into(), json!(1));
            refused_naming(&spec, "bogus");
        }
    }

    #[test]
    fn a_misspelled_optional_key_is_refused() {
        let mut chess = load("./specs/chess.json");
        chess["turns"]["star_at"] = json!(1);
        refused_naming(&chess, "star_at");

        let mut kalah = load("./src/tests/kalah/spec.json");
        kalah["board"]["places"][0]["capacty"] = json!(3);
        refused_naming(&kalah, "capacty");
    }

    #[test]
    fn every_catalogue_spec_survives_a_round_trip() {
        for entry in std::fs::read_dir("./specs").expect("the specs directory") {
            let path = entry.expect("an entry").path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("the spec reads");
            let parsed = parse_game_spec_str(&text)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let written = serde_json::to_string(&parsed).expect("the spec writes");
            parse_game_spec_str(&written)
                .unwrap_or_else(|error| panic!("{} after a round trip: {error}", path.display()));
        }
    }

    /// Asserts `spec` is refused with a message containing `phrase`, serde's own.
    fn refused_saying(form: &str, spec: &Value, phrase: &str) {
        let error = parse_game_spec_str(&spec.to_string())
            .err()
            .unwrap_or_else(|| panic!("{form} should be refused"))
            .to_string();
        assert!(
            error.contains(phrase),
            "{form}: expected {phrase:?} in: {error}"
        );
        assert!(
            !error.contains("old form"),
            "{form}: a plain error: {error}"
        );
    }

    #[test]
    fn the_retired_forms_are_refused_with_serde_messages() {
        let chess = load("./specs/chess.json");
        let rook_repeat = format!("/pieces/{}/moves/0/repeat", rook(&chess));
        let keys: [(&str, &str); 22] = [
            ("", "conditions"),
            ("", "hand"),
            ("", "hand_enabled"),
            ("", "no_opponent_check"),
            ("", "jump_enabled"),
            ("", "mandatory_capture"),
            ("", "allow_pass"),
            ("", "leader"),
            ("", "draw_conditions"),
            ("", "stalemate_loses"),
            ("", "win_conditions"),
            ("", "forbid_repeated_positions"),
            ("", "liberty_capture"),
            ("/pieces/0", "demotes_to"),
            ("/pieces/0", "drop_conditions"),
            ("/pieces/0", "drop_restrictions"),
            ("/pieces/0", "restrictions"),
            ("/pieces/0", "unlimited_supply"),
            ("/pieces/0/moves/0", "actions"),
            ("/pieces/0/moves/0", "conditions"),
            ("/pieces/0/moves/0", "modifiers"),
            ("/pieces/0/moves/0", "side_effects"),
        ];
        for (pointer, key) in keys {
            let mut spec = chess.clone();
            spec.pointer_mut(pointer)
                .and_then(Value::as_object_mut)
                .expect("an object")
                .insert(key.into(), json!(true));
            refused_saying(key, &spec, &format!("unknown field `{key}`"));
        }

        let mut until_as_a_name = chess.clone();
        until_as_a_name
            .pointer_mut(&rook_repeat)
            .and_then(Value::as_object_mut)
            .expect("the repeat")
            .insert("until".into(), json!("NOT_EMPTY"));
        refused_saying("until as a name", &until_as_a_name, "NOT_EMPTY");

        let mut until_as_a_condition = chess.clone();
        until_as_a_condition
            .pointer_mut(&rook_repeat)
            .and_then(Value::as_object_mut)
            .expect("the repeat")
            .insert("until".into(), json!({ "condition": "PIECE_AT" }));
        refused_saying("until as a condition", &until_as_a_condition, "`test`");

        let mut end_turn = chess.clone();
        end_turn["pieces"][0]["moves"][0]["do"]
            .as_array_mut()
            .expect("steps")
            .push(json!({ "effect": "END_TURN" }));
        refused_saying("END_TURN", &end_turn, "unknown variant `END_TURN`");
    }

    #[test]
    fn a_repeat_until_of_null_reads_as_none() {
        let mut chess = load("./specs/chess.json");
        let rook_repeat = format!("/pieces/{}/moves/0/repeat", rook(&chess));
        chess
            .pointer_mut(&rook_repeat)
            .and_then(Value::as_object_mut)
            .expect("the repeat")
            .insert("until".into(), json!(null));
        let parsed = parse_game_spec_str(&chess.to_string()).expect("a null until reads");
        let rook = parsed
            .pieces
            .iter()
            .find(|piece| piece.code == "ROOK")
            .expect("a rook");
        let repeat = rook.moves[0].repeat.as_ref().expect("a repeat");
        assert!(repeat.until.is_none());
    }
}
