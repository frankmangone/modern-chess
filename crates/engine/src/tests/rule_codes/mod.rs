#[cfg(test)]
mod tests {
    use serde_json::{Map, Value};

    use crate::logic::blueprint::test_block::Test;
    use crate::logic::game::ends::Outcome;
    use crate::logic::Game;
    use crate::specs::parse_game_spec;

    fn load(name: &str) -> Game {
        parse_game_spec(&format!("./specs/{name}.json"))
            .map(Game::from_spec)
            .expect("Failed to load spec")
    }

    /// The spec as JSON, for looking at what the rules are written with.
    fn spec_of(name: &str) -> Value {
        let spec = parse_game_spec(&format!("./specs/{name}.json")).expect("Failed to load spec");
        serde_json::to_value(spec).expect("a spec is JSON")
    }

    /// Whether any block inside `value` satisfies `wanted`.
    fn any_block(value: &Value, wanted: &dyn Fn(&Map<String, Value>) -> bool) -> bool {
        match value {
            Value::Object(block) => {
                wanted(block) || block.values().any(|inner| any_block(inner, wanted))
            }
            Value::Array(items) => items.iter().any(|inner| any_block(inner, wanted)),
            _ => false,
        }
    }

    /// The moves of a piece, as written.
    fn moves_of(spec: &Value, code: &str) -> Value {
        let piece = spec["pieces"]
            .as_array()
            .and_then(|pieces| pieces.iter().find(|piece| piece["code"] == code))
            .expect("piece exists");
        piece["moves"].clone()
    }

    #[test]
    fn pawn_promotion_condition_is_named() {
        let moves = moves_of(&spec_of("chess"), "PAWN");
        assert!(any_block(&moves, &|block| block.get("test")
            == Some(&Value::from("IN_ZONE"))
            && block.get("zone") == Some(&Value::from("REACH_END"))));
    }

    #[test]
    fn castling_path_condition_is_built_in() {
        let moves = moves_of(&spec_of("chess"), "KING");
        assert!(any_block(&moves, &|block| block.get("test")
            == Some(&Value::from("COUNT"))
            && block["in"]["region"] == "PATH"));
    }

    #[test]
    fn castling_asks_for_an_empty_target_before_its_first_effect() {
        // A move whose steps start by requiring TARGET empty contributes no threats: castling
        // must not add its landing square to the attack set.
        let moves = moves_of(&spec_of("chess"), "KING");
        let castlings: Vec<&Value> = moves
            .as_array()
            .expect("a list of moves")
            .iter()
            .filter(|mv| {
                any_block(&mv["do"], &|block| {
                    block.get("effect") == Some(&Value::from("MOVE"))
                        && block["from"]["offset"].is_array()
                })
            })
            .collect();
        assert_eq!(castlings.len(), 4, "two castlings for each colour");
        for castling in castlings {
            let first = &castling["do"][0];
            assert_eq!(first["test"], "NOT");
            assert_eq!(first["of"]["test"], "COUNT");
            assert_eq!(first["of"]["in"]["at"]["place"], "TARGET");
        }
    }

    #[test]
    fn an_explosion_is_written_in_steps() {
        let spec = spec_of("atomic");
        let explodes = any_block(&spec["pieces"], &|block| {
            block.get("effect") == Some(&Value::from("FOR_EACH"))
                && any_block(&block["do"], &|inner| {
                    inner.get("effect") == Some(&Value::from("REMOVE"))
                })
        });
        assert!(explodes);
    }

    #[test]
    fn an_end_check_is_parsed() {
        let game = load("king_of_the_hill");
        let first = &game.rules.end_checks[0];
        assert_eq!(first.outcome, Outcome::Win);
        assert!(matches!(first.test, Test::Some { .. }));
    }

    #[test]
    fn declared_zone_is_parsed() {
        let game = load("chess");
        let reach_end = &game.rules.zones["REACH_END"];
        assert!(reach_end.places.contains_key("WHITE"));
        assert!(reach_end.places.contains_key("BLACK"));
    }

    #[test]
    fn drop_condition_is_written_in_the_drop_action() {
        let spec = spec_of("shogi");
        let drop = spec["actions"]
            .as_array()
            .and_then(|actions| actions.iter().find(|action| action["name"] == "DROP"))
            .expect("shogi has a DROP action");
        let no_ally_pawn_on_the_file = any_block(&drop["do"], &|block| {
            block.get("test") == Some(&Value::from("COUNT"))
                && block["in"]["region"] == "LINE"
                && block["where"]["owner"] == "ALLY"
                && block["where"]["codes"] == serde_json::json!(["PAWN"])
        });
        assert!(no_ally_pawn_on_the_file);
    }
}
