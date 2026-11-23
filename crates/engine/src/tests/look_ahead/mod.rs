//! `ATTACKED` with `moves: 2`: a place the attacker can take within two of its moves in a row.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Game, Piece};
    use crate::shared::Position;
    use crate::specs::parse_game_spec_str;
    use crate::tests::support::{
        move_action, moves_from as moves_from_place, onto_a_square_without_an_ally,
        onto_an_empty_square,
    };

    fn at(place: [u8; 2]) -> Position {
        Position::from(format!("{},{}", place[0], place[1]))
    }

    fn moves_from(game: &mut Game, from: [u8; 2]) -> Vec<Position> {
        moves_from_place(game, at(from))
    }

    /// The steps of the walker's move: onto an empty square that `attacked` says is not
    /// attacked (`moves` as written; `None` leaves it out).
    fn walker_steps(moves: Option<u8>, keeping: bool) -> Value {
        let mut attacked = json!({
            "test": "ATTACKED", "in": { "region": "AT", "at": { "place": "TARGET" } }
        });
        if let Some(moves) = moves {
            attacked["moves"] = json!(moves);
        }
        if keeping {
            attacked["keeping"] = json!({ "codes": ["WALKER"] });
        }
        let mut steps = vec![json!({ "test": "NOT", "of": attacked })];
        steps.extend(onto_an_empty_square().as_array().unwrap().clone());
        Value::Array(steps)
    }

    /// 8x8; White's WALKER steps one place east when the target is not attacked; Black has a
    /// KNIGHT, a ROOK, and White an inert BLOCK to be taken or to block.
    fn spec(moves: Option<u8>, keeping: bool) -> Value {
        let knight: Vec<Value> = [
            [1, 2],
            [2, 1],
            [-1, 2],
            [-2, 1],
            [1, -2],
            [2, -1],
            [-1, -2],
            [-2, -1],
        ]
        .iter()
        .enumerate()
        .map(|(id, step)| json!({ "id": id, "step": step, "do": onto_a_square_without_an_ally() }))
        .collect();
        let rook: Vec<Value> = [[1, 0], [-1, 0], [0, 1], [0, -1]]
            .iter()
            .enumerate()
            .map(|(id, step)| {
                json!({ "id": id, "step": step, "repeat": { "loop": true },
                        "do": onto_a_square_without_an_ally() })
            })
            .collect();
        json!({
            "format_version": 1,
            "name": "LOOK_AHEAD",
            "board": { "dimensions": [8, 8] },
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "actions": [move_action()],
            "pieces": [
                { "code": "WALKER", "name": "walker",
                  "moves": [{ "id": 0, "step": [1, 0], "do": walker_steps(moves, keeping) }] },
                { "code": "BLOCK", "name": "block", "moves": [] },
                { "code": "KNIGHT", "name": "knight", "moves": knight },
                { "code": "ROOK", "name": "rook", "moves": rook }
            ]
        })
    }

    fn game(spec: &Value, pieces: &[(&str, &str, [u8; 2])]) -> Game {
        let mut game = Game::from_spec(
            parse_game_spec_str(&spec.to_string()).expect("the look-ahead spec reads"),
        );
        for (player, code, place) in pieces {
            game.state.pieces.insert(
                at(*place),
                Piece::new((*code).to_string(), (*player).to_string()),
            );
        }
        game
    }

    fn refused_naming(spec: &Value, key: &str) {
        let error = parse_game_spec_str(&spec.to_string())
            .err()
            .unwrap_or_else(|| panic!("the spec should be refused for {key}"))
            .to_string();
        assert!(error.contains(key), "the refusal names {key}: {error}");
    }

    /// A walker on [0,3] stepping to [1,3], which a knight on [3,3] reaches in two jumps
    /// ([3,3] to [2,5] to [1,3]) and not in one; and a walker on [0,0] stepping to [1,0], which
    /// that knight needs three jumps for.
    fn two_walkers(moves: Option<u8>) -> Game {
        game(
            &spec(moves, false),
            &[
                ("WHITE", "WALKER", [0, 3]),
                ("WHITE", "WALKER", [0, 0]),
                ("BLACK", "KNIGHT", [3, 3]),
            ],
        )
    }

    #[test]
    fn a_knights_two_jumps_reach_what_one_does_not() {
        let mut one = two_walkers(None);
        assert_eq!(moves_from(&mut one, [0, 3]), vec![at([1, 3])]);
        let mut explicit_one = two_walkers(Some(1));
        assert_eq!(moves_from(&mut explicit_one, [0, 3]), vec![at([1, 3])]);
        let mut two = two_walkers(Some(2));
        assert!(
            moves_from(&mut two, [0, 3]).is_empty(),
            "[1,3] is two jumps from the knight on [3,3]"
        );
    }

    #[test]
    fn a_square_three_moves_away_is_not_attacked_in_two() {
        let mut two = two_walkers(Some(2));
        assert_eq!(
            moves_from(&mut two, [0, 0]),
            vec![at([1, 0])],
            "[1,0] is three jumps from the knight on [3,3]"
        );
    }

    #[test]
    fn what_is_attacked_in_one_is_attacked_in_two() {
        let mut game = game(
            &spec(Some(2), false),
            &[("WHITE", "WALKER", [0, 3]), ("BLACK", "ROOK", [7, 3])],
        );
        assert!(
            moves_from(&mut game, [0, 3]).is_empty(),
            "the rook on [7,3] attacks [1,3] along the row"
        );
    }

    /// The rook on [7,7] reaches row 3 or column 1 (which attack [1,3]) after one move only by
    /// going down its column to [7,3], or along its row to [1,7]. A White block on [2,7] stops
    /// the row at a capture on [2,7], which is on neither.
    #[test]
    fn the_first_move_may_take_a_piece_and_still_attack() {
        let walker = ("WHITE", "WALKER", [0, 3]);
        let rook = ("BLACK", "ROOK", [7, 7]);
        let wall = ("WHITE", "BLOCK", [2, 7]);
        // The rook takes the block on [7,3], then looks along row 3 to [1,3].
        let mut taking = game(
            &spec(Some(2), false),
            &[walker, rook, wall, ("WHITE", "BLOCK", [7, 3])],
        );
        assert!(moves_from(&mut taking, [0, 3]).is_empty());
        // The block is on [7,4] instead: the column stops there, on row 4, and [7,3] is out.
        let mut stopped = game(
            &spec(Some(2), false),
            &[walker, rook, wall, ("WHITE", "BLOCK", [7, 4])],
        );
        assert_eq!(moves_from(&mut stopped, [0, 3]), vec![at([1, 3])]);
    }

    #[test]
    fn moves_must_be_one_or_two() {
        for moves in [0, 3] {
            refused_naming(&spec(Some(moves), false), "1 or 2");
        }
    }

    #[test]
    fn moves_two_is_refused_with_keeping() {
        refused_naming(&spec(Some(2), true), "`moves` 2 with `keeping`");
    }
}
