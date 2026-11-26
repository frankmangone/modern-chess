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

    /// The catalogue's Go, which has a marking stage and the actions of it: the stone's ko
    /// rule is the entry of `every_action` scoped to the action STONE.
    fn go_with_a_stage() -> Value {
        serde_json::from_str(include_str!("../../../specs/go.json")).expect("go reads")
    }

    /// The action `name` of `spec`.
    fn action<'a>(spec: &'a mut Value, name: &str) -> &'a mut Value {
        spec["actions"]
            .as_array_mut()
            .expect("actions")
            .iter_mut()
            .find(|action| action["name"] == name)
            .unwrap_or_else(|| panic!("the spec has the action {name}"))
    }

    fn no_repeat() -> Value {
        json!({ "test": "NOT", "of": { "test": "REPEATS" } })
    }

    #[test]
    fn repeats_is_allowed_in_a_way_that_always_ends_the_turn_even_in_a_game_with_stages() {
        // The ko rule of the stone is scoped to STONE, which ends the turn.
        let parsed = read(&go_with_a_stage());
        assert!(parsed.is_ok(), "{:?}", parsed.err());
    }

    #[test]
    fn repeats_is_refused_in_an_every_action_entry_that_reaches_a_stage_action() {
        // No `for`: the entry applies to AGREE, an action of a stage.
        let mut spec = go_with_a_stage();
        spec["every_action"] = json!([{ "do": [no_repeat()] }]);
        refused_naming(&spec, &["REPEATS", "every_action", "turns.stages", "`for`"]);
        let mut scoped_to_the_stage = go_with_a_stage();
        scoped_to_the_stage["every_action"] = json!([{ "for": ["AGREE"], "do": [no_repeat()] }]);
        refused_naming(&scoped_to_the_stage, &["REPEATS", "AGREE", "stage"]);
        // Scoped to STONE, which ends the turn, it reads.
        let mut scoped_to_the_stone = go_with_a_stage();
        scoped_to_the_stone["every_action"] = json!([{ "for": ["STONE"], "do": [no_repeat()] }]);
        assert!(read(&scoped_to_the_stone).is_ok());
    }

    #[test]
    fn repeats_is_refused_in_an_action_that_runs_a_stage_or_an_ask() {
        for effect in [
            json!({ "effect": "STAGE", "name": "MARKING" }),
            json!({ "effect": "ASK", "players": "OTHERS", "stage": "MARKING" }),
        ] {
            let mut spec = go_with_a_stage();
            let pass = action(&mut spec, "PASS");
            pass["do"] = json!([no_repeat(), effect]);
            refused_naming(&spec, &["REPEATS", "turns.stages"]);
        }
    }

    #[test]
    fn repeats_is_refused_in_an_action_of_a_stage() {
        let mut spec = go_with_a_stage();
        action(&mut spec, "AGREE")["do"] = json!([no_repeat()]);
        refused_naming(&spec, &["REPEATS", "stage"]);
    }

    #[test]
    fn repeats_is_refused_in_a_move_that_runs_a_stage_and_allowed_in_one_that_does_not() {
        let mut spec = checkers();
        // Checkers' jump runs a STAGE; a REPEATS in the move is refused.
        spec["pieces"][0]["moves"][0]["do"]
            .as_array_mut()
            .expect("steps")
            .insert(0, no_repeat());
        refused_naming(&spec, &["REPEATS", "turns.stages"]);
    }

    #[test]
    fn repeats_is_allowed_in_an_end_check_of_a_game_with_stages() {
        let mut spec = checkers();
        spec["end_checks"]
            .as_array_mut()
            .expect("checkers has end checks")
            .push(
                json!({ "when": { "test": "REPEATS", "times": 3 }, "then": { "outcome": "DRAW" } }),
            );
        assert!(
            read(&spec).is_ok(),
            "positions are recorded when a turn is handed on"
        );
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
