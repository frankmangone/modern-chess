//! Pieces on a map: a label step moves to every place the label links to, the neighbourhoods
//! are `next`, positions are place names, and every rule a map cannot mean is refused when the
//! spec is read.
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::{json, Value};

    use crate::logic::{Game, GamePhase, Piece};
    use crate::shared::Position;
    use crate::specs::{parse_game_spec, parse_game_spec_str, GameSpecError};
    use crate::tests::support::{
        choosing, drop, go, move_action, moves_from, onto_a_square_without_an_ally, place_action,
    };

    fn point(name: &str, at: [f64; 2]) -> Value {
        json!({ "name": name, "at": at })
    }

    fn path(name: &str, a: &str, b: &str) -> Value {
        json!({ "name": name, "kind": "path", "ends": [a, b] })
    }

    fn area(name: &str, around: &[&str]) -> Value {
        json!({ "name": name, "kind": "area", "around": around })
    }

    /// Task 2's square map: points `a b c d` with the paths along the sides, the area `f`
    /// inside them, the point `e` and the area `g` around `b e c`, and the written link `a`-`c`.
    fn square_map() -> Value {
        json!({
            "shape": "map",
            "places": [
                point("a", [0.0, 0.0]),
                point("b", [2.0, 0.0]),
                point("c", [2.0, 2.0]),
                point("d", [0.0, 2.0]),
                point("e", [4.0, 1.0]),
                path("ab", "a", "b"),
                path("bc", "b", "c"),
                path("cd", "c", "d"),
                path("da", "d", "a"),
                area("f", &["a", "b", "c", "d"]),
                area("g", &["b", "e", "c"])
            ],
            "links": [["a", "c"]]
        })
    }

    /// A piece that steps along each label of `steps`, moving to an empty place or capturing
    /// an enemy.
    fn walker(steps: &[&str]) -> Value {
        let moves: Vec<Value> = steps
            .iter()
            .enumerate()
            .map(|(id, step)| {
                json!({
                    "id": id,
                    "step": step,
                    "do": onto_a_square_without_an_ally()
                })
            })
            .collect();
        json!({ "code": "WALKER", "name": "walker", "moves": moves })
    }

    fn stone() -> Value {
        json!({ "code": "STONE", "name": "stone", "moves": [] })
    }

    /// A two-player game on the square map; A starts with a walker on each of `walkers`.
    fn spec(steps: &[&str], walkers: &[&str]) -> Value {
        json!({
            "name": "MAP PLAY",
            "board": square_map(),
            "leader": [],
            "turns": { "order": ["A", "B"] },
            "players": [
                { "name": "A", "starting_positions": [{ "piece": "WALKER", "positions": walkers }] },
                { "name": "B", "starting_positions": [] }
            ],
            "pieces": [walker(steps), stone()],
            "actions": [move_action(), place_action("STONE")]
        })
    }

    fn read(spec: &Value) -> Result<Game, GameSpecError> {
        parse_game_spec_str(&spec.to_string()).map(Game::from_spec)
    }

    fn game(spec: &Value) -> Game {
        read(spec).expect("a valid map game")
    }

    fn at(name: &str) -> Position {
        Position::from(name)
    }

    fn put(game: &mut Game, place: &str, code: &str, player: &str) {
        game.state
            .pieces
            .insert(at(place), Piece::new(code.to_string(), player.to_string()));
    }

    /// The places the piece on `from` can move to, sorted by name.
    fn destinations(game: &mut Game, from: &str) -> BTreeSet<String> {
        moves_from(game, at(from))
            .iter()
            .map(|p| p.to_string())
            .collect()
    }

    fn names(list: &[&str]) -> BTreeSet<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn label_step_moves_to_every_linked_place() {
        let mut game = game(&spec(&["next"], &["a"]));
        assert_eq!(destinations(&mut game, "a"), names(&["b", "c", "d"]));
        game.state.phase = choosing();

        put(&mut game, "c", "WALKER", "A");
        assert_eq!(destinations(&mut game, "a"), names(&["b", "d"]));

        go(&mut game, at("b")).expect("b is a destination");
        assert!(game.piece_at_position(&at("a")).is_none());
        let moved = game
            .piece_at_position(&at("b"))
            .expect("the walker is on b");
        assert_eq!(
            (moved.code.as_str(), moved.player.as_str()),
            ("WALKER", "A")
        );
    }

    #[test]
    fn no_link_no_moves() {
        let mut lonely = spec(&["next"], &["z", "a"]);
        lonely["board"]["places"]
            .as_array_mut()
            .unwrap()
            .push(point("z", [9.0, 9.0]));
        let mut game = game(&lonely);
        assert!(destinations(&mut game, "z").is_empty());
        game.state.phase = choosing();
        game.check_game_over();
        assert!(!matches!(game.state.phase, GamePhase::GameOver { .. }));
        assert_eq!(destinations(&mut game, "a"), names(&["b", "c", "d"]));
    }

    #[test]
    fn paths_and_areas_hold_pieces() {
        let mut game = game(&spec(&["next"], &["ab", "f"]));
        assert_eq!(destinations(&mut game, "ab"), names(&["bc", "da"]));
        game.state.phase = choosing();
        assert_eq!(destinations(&mut game, "f"), names(&["g"]));

        let mut game = self::game(&spec(&["areas"], &["a"]));
        assert_eq!(destinations(&mut game, "a"), names(&["f"]));
    }

    /// A walker steps along `next` (point to point) only where a path ending at the place it
    /// lands holds a stone: `AROUND` through `paths` reaches a place `next` never does.
    #[test]
    fn around_through_a_label_counts_the_linked_places() {
        let mut watched = spec(&["next"], &["a"]);
        watched["pieces"][0]["moves"][0]["do"]
            .as_array_mut()
            .unwrap()
            .insert(
                1,
                json!({
                    "test": "COUNT",
                    "in": { "region": "AROUND", "of": { "place": "TARGET" }, "through": "paths" },
                    "where": { "codes": ["STONE"] }
                }),
            );
        let mut game = game(&watched);
        assert!(destinations(&mut game, "a").is_empty(), "no stone anywhere");

        game.state.phase = choosing();
        put(&mut game, "f", "STONE", "B");
        assert!(
            destinations(&mut game, "a").is_empty(),
            "a stone in the area, which is no path"
        );

        game.state.phase = choosing();
        put(&mut game, "cd", "STONE", "B");
        assert_eq!(
            destinations(&mut game, "a"),
            names(&["c", "d"]),
            "the path cd ends at c and at d, not at b"
        );
    }

    #[test]
    fn starting_positions_and_zones_by_name() {
        let mut zoned = spec(&["next"], &["a"]);
        zoned["zones"] = json!([{ "name": "GOAL", "places": { "A": ["c"] } }]);
        let zoned_game = game(&zoned);
        assert!(zoned_game.piece_at_position(&at("a")).is_some());
        assert!(zoned_game.in_zone(&at("c"), "GOAL", "A"));
        assert!(!zoned_game.in_zone(&at("b"), "GOAL", "A"));

        let mut nowhere = zoned.clone();
        nowhere["zones"][0]["places"]["A"] = json!(["c", "nowhere"]);
        let refused = read(&nowhere).expect_err("a zone off the map");
        assert!(matches!(refused, GameSpecError::UnknownPosition(_)));
        assert!(refused.to_string().contains("`nowhere`"), "{refused}");

        let refused = read(&spec(&["next"], &["a", "nowhere"])).expect_err("a start off the map");
        assert!(matches!(refused, GameSpecError::UnknownPosition(_)));
        assert!(refused.to_string().contains("`nowhere`"), "{refused}");

        let chess_with = |position: Value| {
            let mut chess: Value = serde_json::from_str(
                &std::fs::read_to_string("./specs/chess.json").expect("chess spec"),
            )
            .unwrap();
            chess["players"][0]["starting_positions"] =
                json!([{ "piece": "KING", "positions": [position] }]);
            chess["players"][1]["starting_positions"] = json!([]);
            chess
        };
        let by_name = game(&chess_with(json!("4,1")));
        let by_coordinates = game(&chess_with(json!([4, 1])));
        assert_eq!(by_name.state.pieces, by_coordinates.state.pieces);
        assert!(by_name.piece_at_position(&crate::pos![4, 1]).is_some());

        let refused = read(&chess_with(json!("e2"))).expect_err("a name that is no square");
        assert!(matches!(refused, GameSpecError::UnknownPosition(_)));
        let refused = read(&chess_with(json!("9,9"))).expect_err("a square off the board");
        assert!(matches!(refused, GameSpecError::UnknownPosition(_)));
    }

    #[test]
    fn places_named_like_numbers_keep_their_names_through_save_and_restore() {
        let mut numbered = spec(&["next"], &["01", "1, 2"]);
        numbered["board"] = json!({
            "shape": "map",
            "places": [
                point("01", [0.0, 0.0]),
                point("02", [1.0, 0.0]),
                point("1, 2", [2.0, 0.0])
            ],
            "links": [["01", "02"], ["02", "1, 2"]]
        });
        let game = game(&numbered);
        let saved = game.save_state().expect("the state saves");

        let mut restored = self::game(&numbered);
        restored.state.pieces.clear();
        restored.restore_state(&saved).expect("the state restores");

        assert_eq!(restored.state.pieces, game.state.pieces);
        assert!(restored.piece_at_position(&at("01")).is_some());
        assert!(restored.piece_at_position(&at("1, 2")).is_some());
        assert_eq!(destinations(&mut restored, "01"), names(&["02"]));
    }

    #[test]
    fn a_grid_zone_written_as_a_name_must_name_an_enabled_square() {
        let chess_with_zone = |position: Value| {
            let mut chess: Value = serde_json::from_str(
                &std::fs::read_to_string("./specs/chess.json").expect("chess spec"),
            )
            .unwrap();
            chess["zones"]
                .as_array_mut()
                .unwrap()
                .push(json!({ "name": "GOAL", "places": { "WHITE": [position] } }));
            chess
        };
        let refused = read(&chess_with_zone(json!("e4"))).expect_err("a name that is no square");
        assert!(matches!(refused, GameSpecError::UnknownPosition(_)));
        assert!(refused.to_string().contains("`e4`"), "{refused}");
        let refused = read(&chess_with_zone(json!("9,9"))).expect_err("a square off the board");
        assert!(matches!(refused, GameSpecError::UnknownPosition(_)));
        assert!(refused.to_string().contains("`9,9`"), "{refused}");

        let by_name = game(&chess_with_zone(json!("4,1")));
        assert!(by_name.in_zone(&crate::pos![4, 1], "GOAL", "WHITE"));
        game(&chess_with_zone(json!([4, 1])));
        game(&chess_with_zone(json!([9, 9])));
    }

    #[test]
    fn go_liberties_and_connection_on_a_map() {
        let mut go = spec(&[], &[]);
        go["every_action"] = json!([crate::tests::support::liberty_rule(&["STONE"])]);
        let mut game = self::game(&go);
        put(&mut game, "a", "STONE", "A");
        put(&mut game, "b", "STONE", "B");
        put(&mut game, "c", "STONE", "B");
        game.state.current_turn = 1;
        drop(&mut game, "STONE", at("d")).unwrap();
        assert!(
            game.piece_at_position(&at("a")).is_none(),
            "a has no liberty left"
        );
        assert!(game.piece_at_position(&at("d")).is_some());

        let mut connect = spec(&[], &[]);
        connect["zones"] = json!([
            { "name": "WEST", "places": { "A": ["d"] } },
            { "name": "EAST", "places": { "A": ["b"] } }
        ]);
        connect["win_conditions"] = json!([{
            "type": "CONNECTION", "through": "adjacent", "zones": ["WEST", "EAST"],
            "pieces": ["STONE"]
        }]);
        let drop_b = |stones: &[&str]| {
            let mut game = self::game(&connect);
            for &place in stones {
                put(&mut game, place, "STONE", "A");
            }
            drop(&mut game, "STONE", at("b")).unwrap();
            game.state.phase
        };
        let won = GamePhase::GameOver {
            winner: Some("A".to_string()),
        };
        assert_eq!(drop_b(&["d", "a"]), won, "d - a - b is a chain along next");
        assert_ne!(drop_b(&["d"]), won, "d and b are not linked");
    }

    /// Reads `spec` and returns the refusal's message, checking it is a map refusal.
    fn refusal(spec: &Value) -> String {
        let refused = read(spec).expect_err("refused on a map");
        assert!(
            matches!(
                refused,
                GameSpecError::NotOnAMap(_) | GameSpecError::UnknownLabel { .. }
            ),
            "{refused:?}"
        );
        refused.to_string()
    }

    #[test]
    fn refusals_on_a_map() {
        let base = spec(&["next"], &["a"]);
        let first_move = |change: &dyn Fn(&mut Value)| {
            let mut changed = base.clone();
            change(&mut changed["pieces"][0]["moves"][0]);
            refusal(&changed)
        };

        let message = first_move(&|m| m["step"] = json!([0, 1]));
        assert!(message.contains("step"), "{message}");

        let message = first_move(&|m| {
            m["do"].as_array_mut().unwrap().insert(
                1,
                json!({
                    "test": "COUNT",
                    "in": { "region": "AT", "at": { "place": "START", "offset": [0, 1] } }
                }),
            )
        });
        assert!(message.contains("offset"), "{message}");

        // An effect's places take no offset on a map either: where a piece is removed, and
        // where it moves from and to.
        let offset = json!({ "place": "START", "offset": [0, 1] });
        for effect in [
            json!({ "effect": "REMOVE", "at": offset }),
            json!({ "effect": "MOVE", "from": offset, "to": { "place": "TARGET" } }),
            json!({ "effect": "MOVE", "from": { "place": "START" }, "to": offset }),
        ] {
            let message = first_move(&|m| m["do"].as_array_mut().unwrap().push(effect.clone()));
            assert!(message.contains("offset"), "{message}");
        }

        let message = first_move(&|m| m["repeat"] = json!({ "times": 2 }));
        assert!(message.contains("repeat"), "{message}");
        let message = first_move(&|m| m["repeat"] = json!({}));
        assert!(message.contains("repeat"), "{message}");

        let message = first_move(&|m| m["step"] = json!("diagonal"));
        assert!(message.contains("`diagonal`"), "{message}");

        let mut directed = base.clone();
        directed["players"][0]["direction"] = json!([[1, 0], [0, 1]]);
        let message = refusal(&directed);
        assert!(message.contains("direction"), "{message}");
        let mut empty = base.clone();
        empty["players"][1]["direction"] = json!([]);
        let message = refusal(&empty);
        assert!(message.contains("direction"), "{message}");

        let mut on_file = base.clone();
        on_file["actions"][1]["do"].as_array_mut().unwrap().insert(
            2,
            json!({
                "test": "NOT",
                "of": {
                    "test": "COUNT",
                    "in": { "region": "LINE", "through": { "place": "TARGET" }, "along": [0, 1] },
                    "where": { "owner": "ALLY", "codes": ["STONE"] }
                }
            }),
        );
        let message = refusal(&on_file);
        assert!(message.contains("LINE"), "{message}");

        let mut coordinates = base.clone();
        coordinates["players"][0]["starting_positions"][0]["positions"] = json!([[0, 0]]);
        let message = refusal(&coordinates);
        assert!(message.contains("coordinates"), "{message}");

        let mut zone = base.clone();
        zone["zones"] = json!([{ "name": "GOAL", "places": { "A": [[0, 0]] } }]);
        let message = refusal(&zone);
        assert!(message.contains("coordinates"), "{message}");
    }

    #[test]
    fn refusals_on_a_grid() {
        let mut chess: Value = serde_json::from_str(
            &std::fs::read_to_string("./specs/chess.json").expect("chess spec"),
        )
        .unwrap();
        chess["pieces"][0]["moves"][0]["step"] = json!("next");
        let refused = read(&chess).expect_err("a label on a grid");
        assert!(
            matches!(refused, GameSpecError::NotOnAGrid(_)),
            "{refused:?}"
        );
        assert!(refused.to_string().contains("`next`"), "{refused}");

        assert!(parse_game_spec("./specs/chess.json").is_ok());
    }

    #[test]
    fn state_round_trip() {
        let mut game = game(&spec(&["next"], &["a"]));
        destinations(&mut game, "a");
        go(&mut game, at("d")).unwrap();
        let saved = game.save_state().expect("the state saves");

        let mut restored = self::game(&spec(&["next"], &["a"]));
        restored.restore_state(&saved).expect("the state restores");
        assert_eq!(restored.state.pieces, game.state.pieces);
        assert_eq!(restored.state.current_turn, game.state.current_turn);
        assert_eq!(restored.current_player(), "B");
        assert!(restored.piece_at_position(&at("d")).is_some());
    }
}
