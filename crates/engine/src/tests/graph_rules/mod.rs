//! Rules on the board graph where a grid with holes, a piece off the board or a crooked path
//! could tell the graph apart from coordinate arithmetic.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Game, GameTransition, Piece};
    use crate::shared::Position;
    use crate::specs::parse_game_spec_str;

    fn game(dimensions: Value, disabled: Value, pieces: Value, white: Value) -> Game {
        let spec = json!({
            "name": "GRAPH RULES",
            "board": { "dimensions": dimensions, "disabled_positions": disabled },
            "leader": [],
            "turns": { "order": ["WHITE", "BLACK"] },
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": white },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "pieces": pieces
        });
        parse_game_spec_str(&spec.to_string())
            .map(Game::from_spec)
            .expect("the spec should load")
    }

    fn piece(code: &str, step: Value, extra: Value) -> Value {
        let mut movement = json!({
            "id": 0,
            "step": step,
            "actions": [
                { "state": "EMPTY", "action": "MOVE" },
                { "state": "ENEMY", "action": "CAPTURE" }
            ]
        });
        if let (Some(movement), Some(extra)) = (movement.as_object_mut(), extra.as_object()) {
            movement.extend(extra.clone());
        }
        json!({ "code": code, "name": code, "moves": [movement] })
    }

    fn destinations(game: &mut Game, position: Position) -> Vec<Position> {
        game.transition(GameTransition::CalculateMoves { position })
            .unwrap();
        let mut found: Vec<Position> = game
            .state
            .available_moves
            .as_ref()
            .map(|moves| moves.keys().cloned().collect())
            .unwrap_or_default();
        found.sort();
        found
    }

    #[test]
    fn holes_stop_slides_but_not_leaps() {
        let mut game = game(
            json!([5, 1]),
            json!([[2, 0]]),
            json!([
                piece(
                    "ROOK",
                    json!([-1, 0]),
                    json!({ "repeat": { "until": "NOT_EMPTY", "loop": true } })
                ),
                piece("LEAPER", json!([2, 0]), json!({}))
            ]),
            json!([
                { "piece": "ROOK", "positions": [[4, 0]] },
                { "piece": "LEAPER", "positions": [[1, 0]] }
            ]),
        );
        assert_eq!(destinations(&mut game, pos![4, 0]), vec![pos![3, 0]]);
        assert_eq!(destinations(&mut game, pos![1, 0]), vec![pos![3, 0]]);
    }

    #[test]
    fn an_explosion_leaves_holes_alone() {
        let bomb = json!({
            "code": "BOMB",
            "name": "BOMB",
            "moves": [{
                "id": 0,
                "step": [0, -1],
                "actions": [{
                    "state": "ENEMY",
                    "action": "CAPTURE",
                    "side_effects": [{ "action": "EXPLODE" }]
                }]
            }]
        });
        let mut game = game(
            json!([3, 3]),
            json!([[1, 0]]),
            json!([bomb]),
            json!([{ "piece": "BOMB", "positions": [[1, 2]] }]),
        );
        for square in [[1, 1], [0, 0], [2, 0], [0, 1], [2, 1], [0, 2], [2, 2]] {
            game.state.pieces.insert(
                Position::from_coordinates(&square),
                Piece::new("BOMB".to_string(), "BLACK".to_string()),
            );
        }
        game.transition(GameTransition::CalculateMoves {
            position: pos![1, 2],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: pos![1, 1],
        })
        .unwrap();
        assert!(game.state.pieces.is_empty());
    }

    #[test]
    fn a_piece_off_the_board_has_no_moves() {
        let mut game = game(
            json!([8, 8]),
            json!([]),
            json!([piece(
                "ROOK",
                json!([-1, 0]),
                json!({ "repeat": { "until": "NOT_EMPTY", "loop": true } })
            )]),
            json!([{ "piece": "ROOK", "positions": [[8, 0]] }]),
        );
        assert_eq!(destinations(&mut game, pos![8, 0]), Vec::<Position>::new());
    }

    #[test]
    fn a_path_that_is_not_a_line_does_not_exist() {
        let mut game = game(
            json!([8, 8]),
            json!([]),
            json!([piece(
                "CROOK",
                json!([2, 1]),
                json!({ "conditions": [{ "condition": "PATH_PIECE_COUNT", "max": 0 }] })
            )]),
            json!([{ "piece": "CROOK", "positions": [[0, 0]] }]),
        );
        assert_eq!(destinations(&mut game, pos![0, 0]), Vec::<Position>::new());
    }

    /// A hole breaks a line: the king may not castle across a disabled square, as a rook may
    /// not slide across one. (Before boards were graphs, the hole was skipped and the castling
    /// was offered.)
    #[test]
    fn castling_does_not_cross_a_hole() {
        let mut spec: Value =
            serde_json::from_str(&std::fs::read_to_string("./specs/chess.json").unwrap()).unwrap();
        spec["board"]["disabled_positions"] = json!([[1, 0]]);
        for player in spec["players"].as_array_mut().unwrap() {
            for group in player["starting_positions"].as_array_mut().unwrap() {
                group["positions"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|p| ![json!([1, 0]), json!([2, 0]), json!([3, 0])].contains(p));
            }
        }
        let mut game = parse_game_spec_str(&spec.to_string())
            .map(Game::from_spec)
            .expect("chess with a hole should load");
        assert_eq!(destinations(&mut game, pos![4, 0]), vec![pos![3, 0]]);
    }
}
