//! Turn stages: `turns.stages` and the `STAGE` effect keep the turn with the same player, in a
//! stage that limits their ways with a test. The blocks it brings are the place `LAST` and the
//! test `SAME`. What they are refused for is here; the jump lock they replace is in `checkers`,
//! and the free turn in `kalah`.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Game, Piece};
    use crate::specs::{parse_game_spec_str, GameSpec, GameSpecError};
    use crate::tests::support::{go, jumping, pick};

    fn checkers() -> Value {
        serde_json::from_str(include_str!("../../../specs/checkers.json")).expect("checkers reads")
    }

    fn read(spec: &Value) -> Result<GameSpec, GameSpecError> {
        parse_game_spec_str(&spec.to_string())
    }

    fn refused_naming(spec: &Value, naming: &[&str]) {
        let error = read(spec)
            .err()
            .unwrap_or_else(|| panic!("the spec naming {naming:?} should be refused"))
            .to_string();
        for named in naming {
            assert!(error.contains(named), "the refusal names {named}: {error}");
        }
    }

    #[test]
    fn a_stage_effect_must_name_a_declared_stage() {
        let mut spec = checkers();
        spec["pieces"][0]["moves"][0]["do"]
            .as_array_mut()
            .expect("steps")
            .push(json!({ "effect": "STAGE", "name": "NOWHERE" }));
        refused_naming(&spec, &["NOWHERE", "turns.stages"]);
    }

    #[test]
    fn a_stage_name_is_written_once() {
        let mut spec = checkers();
        spec["turns"]["stages"]
            .as_array_mut()
            .expect("stages")
            .push(json!({ "name": "JUMPING" }));
        refused_naming(&spec, &["JUMPING"]);
    }

    #[test]
    fn last_is_a_place_of_steps_and_of_a_stage_but_not_of_must_or_an_end_check() {
        let same_last = json!({
            "test": "SAME", "a": { "place": "START" }, "b": { "place": "LAST" }
        });
        let mut in_must = checkers();
        in_must["must"] = json!([{
            "test": "IN_ZONE", "at": { "place": "LAST" }, "zone": "REACH_END"
        }]);
        refused_naming(&in_must, &["LAST"]);

        let mut in_end = checkers();
        in_end["end_checks"] = json!([{
            "as": "NEXT",
            "when": { "test": "IN_ZONE", "at": { "place": "LAST" }, "zone": "REACH_END" },
            "then": { "outcome": "DRAW" }
        }]);
        refused_naming(&in_end, &["LAST"]);

        let mut in_stage = checkers();
        in_stage["turns"]["stages"][0]["only"] = same_last;
        assert!(read(&in_stage).is_ok());
    }

    #[test]
    fn a_stage_only_cannot_name_target() {
        let mut spec = checkers();
        spec["turns"]["stages"][0]["only"] = json!({
            "test": "SAME", "a": { "place": "TARGET" }, "b": { "place": "LAST" }
        });
        refused_naming(&spec, &["TARGET"]);
    }

    #[test]
    fn a_stage_with_a_misspelled_key_is_refused() {
        let mut spec = checkers();
        spec["turns"]["stages"][0]["onyl"] = json!({ "test": "STUCK" });
        assert!(read(&spec).is_err());
    }

    #[test]
    fn repeats_is_refused_in_a_game_with_stages() {
        let mut spec = checkers();
        spec["must"] = json!([{ "test": "REPEATS" }]);
        refused_naming(&spec, &["REPEATS", "turns.stages"]);
    }

    #[test]
    fn a_path_in_a_stage_only_is_refused_because_no_step_fixes_its_line() {
        let mut spec = checkers();
        spec["turns"]["stages"][0]["only"] = json!({
            "test": "COUNT",
            "in": { "region": "PATH", "from": { "place": "START" }, "to": { "place": "LAST" } },
            "max": 0
        });
        refused_naming(&spec, &["PATH", "stage"]);
    }

    #[test]
    fn last_in_the_until_of_a_repeat_is_refused_because_it_names_no_place() {
        let mut spec: Value =
            serde_json::from_str(include_str!("../../../specs/chess.json")).expect("chess reads");
        let rook = spec["pieces"]
            .as_array_mut()
            .expect("pieces")
            .iter_mut()
            .find(|piece| piece["code"] == "ROOK")
            .expect("a rook");
        rook["moves"][0]["repeat"]["until"] = json!({
            "test": "COUNT", "in": { "region": "AT", "at": { "place": "LAST" } }
        });
        refused_naming(&spec, &["LAST", "repeat"]);
    }

    #[test]
    fn a_move_that_moves_nothing_leaves_last_where_the_latest_move_put_it() {
        // The jump ends with a MOVE from START to START, which moves nothing: LAST stays where
        // the piece landed, so the stage still finds the piece that jumped.
        let mut spec = checkers();
        spec["pieces"][0]["moves"][2]["do"]
            .as_array_mut()
            .expect("the jump's steps")
            .push(json!({
                "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "START" }
            }));
        let mut game = Game::from_spec(read(&spec).expect("the spec reads"));
        game.state.pieces.clear();
        for (place, player) in [
            (pos![3, 1], "RED"),
            (pos![4, 2], "BLACK"),
            (pos![4, 4], "BLACK"),
            (pos![0, 7], "BLACK"),
        ] {
            game.state
                .pieces
                .insert(place, Piece::new("MAN".to_string(), player.to_string()));
        }

        pick(&mut game, pos![3, 1]).unwrap();
        go(&mut game, pos![5, 3]).unwrap();

        assert_eq!(jumping(&game), Some(pos![5, 3]));
    }
}
