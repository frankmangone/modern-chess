//! Specs are proposed by users and a game is rebuilt on every request, so what building and
//! walking the board costs must stay small for any spec the checker accepts.
#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use serde_json::json;

    use crate::logic::{Game, Piece};
    use crate::specs::parse_game_spec_str;
    use crate::tests::support::{move_action, onto_an_empty_square, pick, targets};

    /// Far above what these take, far below what building a table per edge label took.
    const BUDGET: Duration = Duration::from_secs(2);

    #[test]
    fn a_spec_with_thousands_of_moves_builds_quickly() {
        let pieces: Vec<_> = (0..10)
            .map(|piece| {
                let moves: Vec<_> = (0..255)
                    .map(|id| {
                        json!({
                            "id": id,
                            "step": [id - 127, piece + 1],
                            "do": onto_an_empty_square()
                        })
                    })
                    .collect();
                json!({ "code": format!("P{piece}"), "name": "P", "moves": moves })
            })
            .collect();
        let spec = json!({
            "name": "MANY MOVES",
            "board": { "dimensions": [100, 100] },
            "turns": { "order": ["WHITE", "BLACK"] },
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "pieces": pieces,
            "actions": [move_action()]
        });
        let text = spec.to_string();

        let start = Instant::now();
        let mut game = parse_game_spec_str(&text)
            .map(Game::from_spec)
            .unwrap_or_else(|e| panic!("the spec should load: {e:?}"));
        game.state.pieces.insert(
            pos![50, 50],
            Piece::new("P3".to_string(), "WHITE".to_string()),
        );
        pick(&mut game, pos![50, 50]).unwrap();
        let elapsed = start.elapsed();

        assert!(elapsed < BUDGET, "took {elapsed:?}");
        assert!(!targets(&game).is_empty());
    }

    #[test]
    fn an_eight_dimensional_neighbourhood_is_quick() {
        let identity: Vec<Vec<i16>> = (0..8)
            .map(|row| (0..8).map(|col| i16::from(row == col)).collect())
            .collect();
        let mut step = vec![0i16; 8];
        step[0] = 1;
        let spec = json!({
            "name": "EIGHT",
            "board": { "dimensions": [3, 3, 3, 3, 3, 3, 3, 3] },
            "turns": { "order": ["WHITE", "BLACK"] },
            "players": [
                { "name": "WHITE", "direction": identity, "starting_positions": [] },
                { "name": "BLACK", "direction": identity, "starting_positions": [] }
            ],
            "pieces": [{
                "code": "SCOUT",
                "name": "Scout",
                "moves": [{
                    "id": 0,
                    "step": step,
                    "do": [
                        { "test": "NOT", "of": {
                            "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }
                        } },
                        {
                            "test": "COUNT",
                            "in": { "region": "AROUND", "of": { "place": "START" } },
                            "where": { "owner": "ALLY" },
                            "min": 1
                        },
                        { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
                    ]
                }]
            }],
            "actions": [move_action()]
        });

        let start = Instant::now();
        let mut game = parse_game_spec_str(&spec.to_string())
            .map(Game::from_spec)
            .expect("the spec should load");
        let scout = || Piece::new("SCOUT".to_string(), "WHITE".to_string());
        game.state
            .pieces
            .insert(pos![1, 1, 1, 1, 1, 1, 1, 1], scout());
        game.state
            .pieces
            .insert(pos![1, 1, 1, 1, 1, 1, 1, 0], scout());
        pick(&mut game, pos![1, 1, 1, 1, 1, 1, 1, 1]).unwrap();
        let elapsed = start.elapsed();

        assert!(elapsed < BUDGET, "took {elapsed:?}");
        assert_eq!(targets(&game).len(), 1);
    }
}
