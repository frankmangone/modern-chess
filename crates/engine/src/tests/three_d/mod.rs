//! A small original 3D game (rooks, kings, a bomb and a scout on a 4×4×4 board) that proves
//! the engine plays boards of more than two dimensions: lines along every axis, check, mate,
//! explosions and neighbour counts.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::shared::Position;
    use crate::specs::parse_game_spec_str;

    /// The 6 unit steps along one axis.
    fn axes() -> Vec<Vec<i16>> {
        all_steps()
            .into_iter()
            .filter(|step| step.iter().filter(|&&d| d != 0).count() == 1)
            .collect()
    }

    /// The 26 non-zero steps in {-1, 0, 1}³.
    fn all_steps() -> Vec<Vec<i16>> {
        let mut steps = Vec::new();
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if (x, y, z) != (0, 0, 0) {
                        steps.push(vec![x, y, z]);
                    }
                }
            }
        }
        steps
    }

    fn moves(steps: Vec<Vec<i16>>, extra: Value) -> Value {
        steps
            .into_iter()
            .enumerate()
            .map(|(id, step)| {
                let mut movement = json!({ "id": id, "step": step });
                if let (Some(movement), Some(extra)) = (movement.as_object_mut(), extra.as_object())
                {
                    movement.extend(extra.clone());
                }
                movement
            })
            .collect()
    }

    fn load() -> Game {
        let move_or_capture = json!([
            { "state": "EMPTY", "action": "MOVE" },
            { "state": "ENEMY", "action": "CAPTURE" }
        ]);
        let spec = json!({
            "name": "CUBE",
            "board": { "dimensions": [4, 4, 4] },
            "leader": ["KING"],
            "turns": { "order": ["WHITE", "BLACK"] },
            "players": [
                {
                    "name": "WHITE",
                    "direction": [[1, 0, 0], [0, 1, 0], [0, 0, 1]],
                    "starting_positions": [{ "piece": "KING", "positions": [[0, 0, 0]] }]
                },
                {
                    "name": "BLACK",
                    "direction": [[-1, 0, 0], [0, -1, 0], [0, 0, 1]],
                    "starting_positions": [{ "piece": "KING", "positions": [[3, 3, 3]] }]
                }
            ],
            "pieces": [
                {
                    "code": "ROOK",
                    "name": "Rook",
                    "moves": moves(axes(), json!({
                        "actions": move_or_capture,
                        "repeat": { "loop": true }
                    }))
                },
                {
                    "code": "KING",
                    "name": "King",
                    "moves": moves(all_steps(), json!({
                        "actions": move_or_capture,
                        "conditions": [{
                            "test": "NOT",
                            "of": {
                                "test": "ATTACKED",
                                "in": { "region": "AT", "at": { "place": "TARGET" } }
                            }
                        }]
                    }))
                },
                {
                    "code": "BOMB",
                    "name": "Bomb",
                    "moves": moves(axes(), json!({
                        "actions": [{
                            "state": "ENEMY",
                            "action": "CAPTURE",
                            "side_effects": [{ "action": "EXPLODE" }]
                        }]
                    }))
                },
                {
                    "code": "SCOUT",
                    "name": "Scout",
                    "moves": moves(vec![vec![1, 0, 0]], json!({
                        "actions": [{ "state": "EMPTY", "action": "MOVE" }],
                        "conditions": [{
                            "test": "COUNT",
                            "in": { "region": "AROUND", "of": { "place": "START" } },
                            "where": { "owner": "ALLY" },
                            "min": 1
                        }]
                    }))
                }
            ]
        });
        parse_game_spec_str(&spec.to_string())
            .map(Game::from_spec)
            .expect("the 3D spec should load")
    }

    fn insert(game: &mut Game, position: Position, code: &str, player: &str) {
        game.state
            .pieces
            .insert(position, Piece::new(code.to_string(), player.to_string()));
    }

    fn moves_for(game: &mut Game, position: Position) -> Vec<Position> {
        game.transition(GameTransition::CalculateMoves { position })
            .unwrap();
        let mut destinations: Vec<Position> = game
            .state
            .available_moves
            .as_ref()
            .map(|moves| moves.keys().cloned().collect())
            .unwrap_or_default();
        destinations.sort();
        destinations
    }

    fn play(game: &mut Game, from: Position, to: Position) {
        game.transition(GameTransition::CalculateMoves { position: from })
            .unwrap();
        game.transition(GameTransition::ExecuteMove { position: to })
            .unwrap();
    }

    #[test]
    fn a_rook_slides_along_all_three_axes() {
        let mut game = load();
        insert(&mut game, pos![1, 1, 1], "ROOK", "WHITE");

        let mut expected = Vec::new();
        for value in [0, 2, 3] {
            expected.push(pos![value, 1, 1]);
            expected.push(pos![1, value, 1]);
            expected.push(pos![1, 1, value]);
        }
        expected.sort();
        assert_eq!(moves_for(&mut game, pos![1, 1, 1]), expected);
    }

    #[test]
    fn a_king_may_not_step_onto_a_line_attacked_along_the_third_axis() {
        let mut game = load();
        insert(&mut game, pos![1, 1, 3], "ROOK", "BLACK");

        assert_eq!(
            moves_for(&mut game, pos![0, 0, 0]),
            vec![
                pos![0, 0, 1],
                pos![0, 1, 0],
                pos![0, 1, 1],
                pos![1, 0, 0],
                pos![1, 0, 1]
            ]
        );
    }

    #[test]
    fn four_rooks_mate_a_cornered_king() {
        let mut game = load();
        for position in [pos![0, 2, 2], pos![0, 2, 3], pos![0, 3, 2], pos![1, 0, 3]] {
            insert(&mut game, position, "ROOK", "WHITE");
        }

        play(&mut game, pos![1, 0, 3], pos![1, 3, 3]);

        assert_eq!(
            game.state.phase,
            GamePhase::GameOver {
                winner: Some("WHITE".to_string())
            }
        );
    }

    #[test]
    fn an_explosion_reaches_all_26_neighbours() {
        let mut game = load();
        game.state.pieces.remove(&pos![3, 3, 3]);
        insert(&mut game, pos![0, 3, 0], "KING", "BLACK");
        insert(&mut game, pos![2, 2, 1], "BOMB", "WHITE");
        let cube: Vec<Position> = (1..=3u8)
            .flat_map(|x| (1..=3u8).flat_map(move |y| (1..=3u8).map(move |z| pos![x, y, z])))
            .collect();
        for position in &cube {
            if *position != pos![2, 2, 1] {
                insert(&mut game, position.clone(), "ROOK", "BLACK");
            }
        }

        play(&mut game, pos![2, 2, 1], pos![2, 2, 2]);

        let survivors: Vec<&Position> = cube
            .iter()
            .filter(|position| game.state.pieces.contains_key(*position))
            .collect();
        assert!(survivors.is_empty(), "left standing: {survivors:?}");
        assert!(game.state.pieces.contains_key(&pos![0, 0, 0]));
        assert!(game.state.pieces.contains_key(&pos![0, 3, 0]));
    }

    #[test]
    fn an_ally_one_step_away_on_the_third_axis_counts_as_adjacent() {
        let mut game = load();
        // Two squares from both kings, so the rook is the scout's only neighbour.
        insert(&mut game, pos![2, 1, 1], "SCOUT", "WHITE");
        insert(&mut game, pos![3, 2, 2], "ROOK", "WHITE");
        assert_eq!(moves_for(&mut game, pos![2, 1, 1]), vec![pos![3, 1, 1]]);

        game.set_idle_state();
        game.state.pieces.remove(&pos![3, 2, 2]);
        assert!(moves_for(&mut game, pos![2, 1, 1]).is_empty());
    }
}
