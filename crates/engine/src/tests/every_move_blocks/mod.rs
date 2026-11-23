//! The tests that look at a way as a whole: `REMOVED` (what the way has taken off the board so
//! far), `REPEATS` (the position it leaves has been seen after an earlier turn) and `ATTACKED`
//! by the mover's own pieces, with their refusals when the spec is read.
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::collections::HashMap;

    use serde_json::{json, Value};

    use crate::logic::{Answer, Game, Piece, Way};
    use crate::shared::Position;
    use crate::specs::{parse_game_spec_str, GameSpec, GameSpecError};

    const WHITE: &str = "WHITE";
    const BLACK: &str = "BLACK";

    fn read(spec: &Value) -> Result<GameSpec, GameSpecError> {
        parse_game_spec_str(&spec.to_string())
    }

    fn game_of(spec: &Value) -> Game {
        Game::from_spec(read(spec).unwrap_or_else(|error| panic!("the spec is read: {error}")))
    }

    fn at(place: &str) -> Value {
        json!({ "place": place })
    }

    /// No ally stands on `TARGET`.
    fn not_ally() -> Value {
        json!({ "test": "NOT", "of": {
            "test": "COUNT",
            "in": { "region": "AT", "at": at("TARGET") },
            "where": { "owner": "ALLY" }
        } })
    }

    /// The piece at `START` goes to `TARGET`.
    fn move_effect() -> Value {
        json!({ "effect": "MOVE", "from": at("START"), "to": at("TARGET") })
    }

    /// `REMOVED where filter`.
    fn removed(filter: Value) -> Value {
        json!({ "test": "REMOVED", "where": filter })
    }

    /// A 4×4 game: a PROBE steps [0, 1] and a GUARD steps [-1, 0], each onto a square no ally
    /// holds; a PAWN does not move. `actions` are the game's actions.
    fn small_spec(actions: Value) -> Value {
        json!({
            "name": "EVERY_MOVE",
            "board": { "dimensions": [4, 4] },
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "pieces": [
                {
                    "code": "PROBE",
                    "name": "probe",
                    "moves": [{ "id": 0, "step": [0, 1], "do": [not_ally(), move_effect()] }]
                },
                {
                    "code": "GUARD",
                    "name": "guard",
                    "moves": [{ "id": 0, "step": [-1, 0], "do": [not_ally(), move_effect()] }]
                },
                { "code": "PAWN", "name": "pawn", "moves": [] }
            ],
            "actions": actions
        })
    }

    /// The action `A`: choose an ally piece, one of its moves, then the test `closing`.
    fn move_then(closing: Value) -> Value {
        small_spec(json!([{ "name": "A", "do": [
            { "choose": "PIECE", "where": { "owner": "ALLY" } },
            { "choose": "MOVE" },
            closing
        ] }]))
    }

    /// The action `A`: choose a place of the board, then `steps`.
    fn place_then(steps: Value) -> Value {
        let mut all = vec![json!({ "choose": "PLACE", "in": { "region": "BOARD" } })];
        all.extend(steps.as_array().cloned().unwrap_or_default());
        small_spec(json!([{ "name": "A", "do": all }]))
    }

    fn put(game: &mut Game, at: Position, code: &str, player: &str) {
        game.state
            .pieces
            .insert(at, Piece::new(code.to_string(), player.to_string()));
    }

    /// The first answer of each way, a place.
    fn firsts(ways: &[Way]) -> BTreeSet<Position> {
        ways.iter()
            .filter_map(|way| match way.answers.first() {
                Some(Answer::Place(place)) => Some(place.clone()),
                _ => None,
            })
            .collect()
    }

    /// The second answer of each way, a place.
    fn seconds(ways: &[Way]) -> BTreeSet<Position> {
        ways.iter()
            .filter_map(|way| match way.answers.get(1) {
                Some(Answer::Place(place)) => Some(place.clone()),
                _ => None,
            })
            .collect()
    }

    fn places(names: &[Position]) -> BTreeSet<Position> {
        names.iter().cloned().collect()
    }

    /// A WHITE PROBE on 0,0 that can take the BLACK PAWN on 0,1, and one on 2,0 whose only move
    /// is quiet.
    fn a_capture_and_a_quiet_move(spec: &Value) -> Game {
        let mut game = game_of(spec);
        put(&mut game, pos![0, 0], "PROBE", WHITE);
        put(&mut game, pos![0, 1], "PAWN", BLACK);
        put(&mut game, pos![2, 0], "PROBE", WHITE);
        game
    }

    #[test]
    fn removed_holds_after_a_way_takes_a_matching_piece() {
        let game = a_capture_and_a_quiet_move(&move_then(removed(
            json!({ "owner": "ENEMY", "codes": ["PAWN"] }),
        )));
        assert_eq!(
            firsts(&game.compute_ways()),
            places(&[pos![0, 0]]),
            "the capture stays and the quiet move is gone"
        );

        let game = a_capture_and_a_quiet_move(&move_then(removed(
            json!({ "owner": "ENEMY", "codes": ["GUARD"] }),
        )));
        assert!(
            game.compute_ways().is_empty(),
            "a PAWN was taken, not a GUARD"
        );
    }

    #[test]
    fn removed_counts_a_piece_taken_by_remove_move_and_place() {
        let enemy_pawn = || removed(json!({ "owner": "ENEMY", "codes": ["PAWN"] }));
        let by_remove = place_then(json!([
            { "effect": "REMOVE", "at": at("TARGET") },
            enemy_pawn()
        ]));
        let by_place = place_then(json!([
            { "effect": "PLACE", "at": at("TARGET"), "code": "PROBE" },
            enemy_pawn()
        ]));
        for (effect, spec) in [("REMOVE", by_remove), ("PLACE", by_place)] {
            let mut game = game_of(&spec);
            put(&mut game, pos![1, 1], "PAWN", BLACK);
            put(&mut game, pos![2, 2], "PAWN", WHITE);
            assert_eq!(
                firsts(&game.compute_ways()),
                places(&[pos![1, 1]]),
                "{effect} onto the enemy PAWN is the one way"
            );
        }

        let by_move = move_then(enemy_pawn());
        let game = a_capture_and_a_quiet_move(&by_move);
        assert_eq!(
            firsts(&game.compute_ways()),
            places(&[pos![0, 0]]),
            "MOVE onto the enemy PAWN"
        );

        // A game where the taken piece goes to the taker's hand: it was still removed.
        let mut to_hand = by_move.clone();
        to_hand["hand_enabled"] = json!(true);
        let game = a_capture_and_a_quiet_move(&to_hand);
        assert_eq!(
            firsts(&game.compute_ways()),
            places(&[pos![0, 0]]),
            "a piece that goes to the hand"
        );

        // Every stone a FOR_EACH removes counts too.
        let by_for_each = place_then(json!([
            {
                "effect": "FOR_EACH",
                "in": { "region": "AROUND", "of": at("TARGET"), "through": "orthogonal" },
                "do": [{ "effect": "REMOVE", "at": at("EACH") }]
            },
            enemy_pawn()
        ]));
        let mut game = game_of(&by_for_each);
        put(&mut game, pos![0, 0], "PAWN", BLACK);
        assert_eq!(
            firsts(&game.compute_ways()),
            places(&[pos![1, 0], pos![0, 1]]),
            "the places orthogonally next to the enemy PAWN"
        );
    }

    #[test]
    fn removed_does_not_hold_for_an_ally_filter_when_an_enemy_was_taken() {
        let game = a_capture_and_a_quiet_move(&move_then(removed(json!({ "owner": "ALLY" }))));
        assert!(game.compute_ways().is_empty(), "{:?}", game.compute_ways());

        let game = a_capture_and_a_quiet_move(&move_then(removed(json!({ "owner": "ANY" }))));
        assert_eq!(firsts(&game.compute_ways()), places(&[pos![0, 0]]));
    }

    /// Two WHITE PROBEs, on 0,0 and 2,0, and the key of the position after the one on 0,0
    /// steps to 0,1, with BLACK to move, recorded as if seen after an earlier turn.
    fn a_repeating_move(spec: &Value) -> Game {
        let mut game = game_of(spec);
        put(&mut game, pos![0, 0], "PROBE", WHITE);
        put(&mut game, pos![2, 0], "PROBE", WHITE);
        let mut moved = Piece::new("PROBE".to_string(), WHITE.to_string());
        moved.total_moves = 1;
        let after: HashMap<Position, Piece> = HashMap::from([
            (pos![0, 1], moved),
            (
                pos![2, 0],
                Piece::new("PROBE".to_string(), WHITE.to_string()),
            ),
        ]);
        let key = game.position_key_for(&after, BLACK);
        game.state.position_hashes = vec![key];
        game
    }

    #[test]
    fn repeats_holds_when_the_position_was_recorded_after_an_earlier_turn() {
        let game = a_repeating_move(&move_then(json!({ "test": "REPEATS" })));
        assert_eq!(firsts(&game.compute_ways()), places(&[pos![0, 0]]));

        let game = a_repeating_move(&move_then(
            json!({ "test": "NOT", "of": { "test": "REPEATS" } }),
        ));
        assert_eq!(
            firsts(&game.compute_ways()),
            places(&[pos![2, 0]]),
            "NOT REPEATS removes the repeating way and no other"
        );
    }

    #[test]
    fn repeats_after_a_drop_leaves_out_the_piece_in_the_hand() {
        let mut spec = small_spec(json!([{ "name": "DROP", "do": [
            { "choose": "HAND" },
            { "choose": "PLACE", "in": { "region": "BOARD" } },
            move_effect(),
            { "test": "NOT", "of": { "test": "REPEATS" } }
        ] }]));
        spec["hand_enabled"] = json!(true);
        let mut game = game_of(&spec);
        game.state
            .hand
            .insert(WHITE.to_string(), HashMap::from([("PROBE".to_string(), 1)]));
        let after = HashMap::from([(
            pos![0, 1],
            Piece::new("PROBE".to_string(), WHITE.to_string()),
        )]);
        game.state.position_hashes = vec![game.position_key_for(&after, BLACK)];

        let ways = game.compute_ways();
        assert_eq!(ways.len(), 15, "every place but the one that repeats");
        assert!(!seconds(&ways).contains(&pos![0, 1]));
    }

    #[test]
    fn repeats_is_false_in_a_game_that_has_recorded_nothing() {
        let mut game = a_repeating_move(&move_then(json!({ "test": "REPEATS" })));
        game.state.position_hashes.clear();
        assert!(game.compute_ways().is_empty(), "{:?}", game.compute_ways());
    }

    #[test]
    fn attacked_by_mover_looks_at_the_movers_own_attacks() {
        // The ally PAWN, which attacks nothing, goes to a place of the board that `by`
        // attacks. The test is judged before the PAWN moves: an attack set leaves out the
        // places its own side holds, so once the PAWN stands on TARGET no ally attacks it.
        let to_a_place_attacked_by = |by: &str| {
            small_spec(json!([{ "name": "A", "do": [
                { "choose": "PIECE", "where": { "owner": "ALLY", "codes": ["PAWN"] } },
                { "choose": "PLACE", "in": { "region": "BOARD" } },
                { "test": "ATTACKED", "in": { "region": "AT", "at": at("TARGET") }, "by": by },
                move_effect()
            ] }]))
        };
        // WHITE's GUARD on 1,1 attacks 0,1; BLACK's PROBE on 2,2 attacks 2,1.
        let position = |spec: &Value| {
            let mut game = game_of(spec);
            put(&mut game, pos![3, 3], "PAWN", WHITE);
            put(&mut game, pos![1, 1], "GUARD", WHITE);
            put(&mut game, pos![2, 2], "PROBE", BLACK);
            game
        };

        let by_mover = position(&to_a_place_attacked_by("MOVER")).compute_ways();
        assert_eq!(
            seconds(&by_mover),
            places(&[pos![0, 1]]),
            "by MOVER, the square WHITE's GUARD attacks and not the one BLACK attacks"
        );

        let by_enemy = position(&to_a_place_attacked_by("ENEMY")).compute_ways();
        assert_eq!(
            seconds(&by_enemy),
            places(&[pos![2, 1]]),
            "by ENEMY, only BLACK's attack"
        );
    }

    /// Asserts that `spec` is refused while it is checked, with a message containing each of
    /// `naming`.
    fn assert_refused(spec: &Value, naming: &[&str]) {
        let error = read(spec)
            .err()
            .unwrap_or_else(|| panic!("the spec naming {naming:?} should be refused"));
        for named in naming {
            assert!(
                error.to_string().contains(named),
                "the refusal names {named}: {error}"
            );
        }
    }

    #[test]
    fn refusals_when_the_spec_is_read() {
        let attacked_by_king = move_then(json!({
            "test": "ATTACKED", "in": { "region": "AT", "at": at("TARGET") }, "by": "KING"
        }));
        assert_refused(&attacked_by_king, &["KING", "action A"]);
        assert!(!matches!(
            read(&attacked_by_king),
            Err(GameSpecError::JsonError(_))
        ));

        assert_refused(&move_then(json!({ "test": "REMOVED" })), &["where"]);
        assert_refused(
            &move_then(json!({ "test": "REPEATS", "where": { "owner": "ENEMY" } })),
            &["where"],
        );
        assert_refused(
            &move_then(removed(json!({ "owner": "FRIEND" }))),
            &["FRIEND"],
        );

        let mut repeats_with_a_jump_lock = move_then(json!({ "test": "REPEATS" }));
        repeats_with_a_jump_lock["jump_enabled"] = json!(true);
        assert_refused(&repeats_with_a_jump_lock, &["REPEATS", "jump_enabled"]);
        let nested = {
            let mut spec = move_then(json!({ "test": "NOT", "of": { "test": "REPEATS" } }));
            spec["jump_enabled"] = json!(true);
            spec
        };
        assert_refused(&nested, &["REPEATS", "jump_enabled"]);

        for fine in [
            move_then(json!({ "test": "REPEATS" })),
            move_then(removed(json!({ "owner": "ENEMY" }))),
            move_then(json!({
                "test": "ATTACKED", "in": { "region": "AT", "at": at("TARGET") }, "by": "MOVER"
            })),
        ] {
            assert!(read(&fine).is_ok(), "{:?}", read(&fine).err());
        }
    }
}
