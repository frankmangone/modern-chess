//! Sides with their own rules: the `PLAYER` test and a reserve for each side. How they play is
//! proved by Siege, in the session crate's tests; this is what the spec may and may not say.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::Game;
    use crate::specs::{parse_game_spec_str, GameSpecError};

    fn read(spec: &Value) -> Result<crate::specs::GameSpec, GameSpecError> {
        parse_game_spec_str(&spec.to_string())
    }

    fn accepted(spec: &Value) {
        if let Err(error) = read(spec) {
            panic!("the spec is refused: {error}");
        }
    }

    fn refusal(spec: &Value) -> String {
        read(spec).expect_err("the spec is refused").to_string()
    }

    /// Two players, a bag each, one piece, and `extra` merged over the top.
    fn spec(extra: Value) -> Value {
        let mut spec = json!({
            "format_version": 1,
            "name": "SIDES",
            "board": { "dimensions": [2, 2] },
            "players": [
                { "name": "A", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "B", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["A", "B"] },
            "pieces": [{ "code": "P", "name": "p", "moves": [] }],
            "bags": [{ "name": "HAND", "for": "EACH_PLAYER" }],
            "actions": [{ "name": "GO", "do": [] }]
        });
        if let (Some(spec), Some(extra)) = (spec.as_object_mut(), extra.as_object()) {
            spec.extend(extra.clone());
        }
        spec
    }

    fn hand_start(start: Value) -> Value {
        spec(json!({ "bags": [{ "name": "HAND", "for": "EACH_PLAYER", "start": start }] }))
    }

    fn held(game: &Game, player: &str) -> Vec<(String, u32)> {
        game.bags()
            .into_iter()
            .find(|bag| bag.name == "HAND" && bag.owner.as_deref() == Some(player))
            .map(|bag| bag.pieces.into_iter().collect())
            .unwrap_or_default()
    }

    // ---- PLAYER

    #[test]
    fn player_is_a_test_wherever_a_test_goes() {
        let only_a = json!({ "test": "PLAYER", "is": ["A"] });
        accepted(&spec(
            json!({ "actions": [{ "name": "GO", "do": [only_a.clone()] }] }),
        ));
        accepted(&spec(json!({ "must": [{ "test": "ANY", "of": [
            { "test": "NOT", "of": only_a.clone() },
            { "test": "REMOVED", "where": { "owner": "ENEMY" } }
        ] }] })));
        accepted(&spec(
            json!({ "every_action": [{ "do": [only_a.clone()] }] }),
        ));
        accepted(&spec(json!({ "end_checks": [
            { "when": only_a, "players": ["A"], "then": { "outcome": "WIN" } }
        ] })));
    }

    #[test]
    fn player_refuses_what_could_not_hold() {
        let empty = refusal(&spec(json!({ "actions": [
            { "name": "GO", "do": [{ "test": "PLAYER", "is": [] }] }
        ] })));
        assert!(
            empty.contains("PLAYER") && empty.contains("empty"),
            "{empty}"
        );
        let stranger = refusal(&spec(json!({ "actions": [
            { "name": "GO", "do": [{ "test": "PLAYER", "is": ["A", "C"] }] }
        ] })));
        assert!(
            stranger.contains('C') && stranger.contains("not a player"),
            "{stranger}"
        );
        // A field it does not have, and no `is`, are refused as everywhere.
        assert!(read(&spec(json!({ "actions": [
            { "name": "GO", "do": [{ "test": "PLAYER", "is": ["A"], "of": "MOVER" }] }
        ] })))
        .is_err());
        assert!(read(&spec(json!({ "actions": [
            { "name": "GO", "do": [{ "test": "PLAYER" }] }
        ] })))
        .is_err());
    }

    // ---- a reserve for each side

    #[test]
    fn a_bag_may_start_with_something_different_for_each_player() {
        let spec = hand_start(json!({ "A": { "P": 3 }, "B": {} }));
        accepted(&spec);
        let game = Game::from_spec(read(&spec).unwrap());
        assert_eq!(held(&game, "A"), vec![("P".to_string(), 3)]);
        assert!(held(&game, "B").is_empty());
    }

    #[test]
    fn a_player_left_out_starts_empty_and_the_plain_form_is_the_same_for_all() {
        let game = Game::from_spec(read(&hand_start(json!({ "B": { "P": 2 } }))).unwrap());
        assert!(held(&game, "A").is_empty());
        assert_eq!(held(&game, "B"), vec![("P".to_string(), 2)]);
        let same = Game::from_spec(read(&hand_start(json!({ "P": 2 }))).unwrap());
        assert_eq!(held(&same, "A"), held(&same, "B"));
        assert_eq!(held(&same, "A"), vec![("P".to_string(), 2)]);
    }

    #[test]
    fn the_per_player_start_is_refused_where_it_means_nothing() {
        // A shared bag has no owner to start for.
        let shared = refusal(&spec(json!({ "bags": [
            { "name": "TRAY", "start": { "A": { "P": 1 } } }
        ] })));
        assert!(
            shared.contains("TRAY") && shared.contains("shared"),
            "{shared}"
        );
        let stranger = refusal(&hand_start(json!({ "C": { "P": 1 } })));
        assert!(
            stranger.contains('C') && stranger.contains("not a player"),
            "{stranger}"
        );
        let unknown = refusal(&hand_start(json!({ "A": { "Z": 1 } })));
        assert!(unknown.contains('Z'), "{unknown}");
        let zero = refusal(&hand_start(json!({ "A": { "P": 0 } })));
        assert!(zero.contains("0 of P"), "{zero}");
        // The capacity binds each player's own start.
        let over = refusal(&spec(json!({ "bags": [
            { "name": "HAND", "for": "EACH_PLAYER", "capacity": 2, "start": { "A": { "P": 3 } } }
        ] })));
        assert!(over.contains("capacity"), "{over}");
        // Mixing the two forms in one start is not a start at all.
        assert!(read(&hand_start(json!({ "A": { "P": 1 }, "P": 1 }))).is_err());
    }

    #[test]
    fn the_start_round_trips_in_the_form_it_was_written() {
        for start in [json!({ "P": 2 }), json!({ "A": { "P": 3 }, "B": {} })] {
            let read = read(&hand_start(start.clone())).unwrap();
            let written = serde_json::to_value(&read).unwrap();
            assert_eq!(written["bags"][0]["start"], start);
        }
    }
}
