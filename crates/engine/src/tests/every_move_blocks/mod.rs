//! The tests that look at a way as a whole: `REMOVED` (what the way has taken off the board so
//! far), `REPEATS` (the position it leaves has been seen after an earlier turn) and `ATTACKED`
//! by the mover's own pieces; the regions `BOARD where`, `GROUP` and `AROUND` of a region, and
//! the filter `empty`; with their refusals when the spec is read.
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::collections::HashMap;

    use serde_json::{json, Value};

    use crate::logic::blueprint::test_block::{judge, Test, TestContext};
    use crate::logic::{Answer, Game, GameTransition, Labels, Piece, Way};
    use crate::shared::Position;
    use crate::specs::game::test_block::TestSpec;
    use crate::specs::{parse_game_spec, parse_game_spec_str, GameSpec, GameSpecError};

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
    /// holds; a PAWN and a KING do not move. `actions` are the game's actions.
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
                { "code": "PAWN", "name": "pawn", "moves": [] },
                { "code": "KING", "name": "king", "moves": [] }
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

    /// The region `GROUP` of `of`, through `through`, with the filter `filter` when written.
    fn group(of: &str, through: &str, filter: Option<Value>) -> Value {
        let mut region = json!({ "region": "GROUP", "of": at(of), "through": through });
        if let Some(filter) = filter {
            region["where"] = filter;
        }
        region
    }

    /// The region `AROUND` the region `of`, through `through`.
    fn around(of: Value, through: &str) -> Value {
        json!({ "region": "AROUND", "of": of, "through": through })
    }

    /// `COUNT in region where filter`, between `min` and `max`.
    fn count(region: Value, filter: Option<Value>, min: u16, max: u16) -> Value {
        let mut test = json!({ "test": "COUNT", "in": region, "min": min, "max": max });
        if let Some(filter) = filter {
            test["where"] = filter;
        }
        test
    }

    /// The action `A`: choose an ally PAWN, then `steps`.
    fn pawn_then(steps: Value) -> Value {
        let mut all =
            vec![json!({ "choose": "PIECE", "where": { "owner": "ALLY", "codes": ["PAWN"] } })];
        all.extend(steps.as_array().cloned().unwrap_or_default());
        small_spec(json!([{ "name": "A", "do": all }]))
    }

    /// A game of `spec` with each of `pieces` put on the board: a place, a code and an owner.
    fn with_pieces(spec: &Value, pieces: &[(Position, &str, &str)]) -> Game {
        let mut game = game_of(spec);
        for (place, code, player) in pieces {
            put(&mut game, place.clone(), code, player);
        }
        game
    }

    #[test]
    fn board_with_a_filter_lists_only_the_places_of_matching_pieces() {
        let ally_king_attacked = place_then(json!([{
            "test": "ATTACKED",
            "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": ["KING"] } }
        }]));
        // BLACK's GUARD on 0,1 attacks 1,1.
        let king_attacked = with_pieces(
            &ally_king_attacked,
            &[
                (pos![0, 1], "GUARD", BLACK),
                (pos![1, 1], "KING", WHITE),
                (pos![3, 3], "PAWN", WHITE),
            ],
        );
        assert_eq!(
            king_attacked.compute_ways().len(),
            16,
            "the ally KING is attacked, so the test holds for every place"
        );

        let pawn_attacked = with_pieces(
            &ally_king_attacked,
            &[
                (pos![0, 1], "GUARD", BLACK),
                (pos![1, 1], "PAWN", WHITE),
                (pos![3, 3], "KING", WHITE),
            ],
        );
        assert!(
            pawn_attacked.compute_ways().is_empty(),
            "an attacked ally PAWN is not in the region"
        );

        // `empty` lists the places that hold no piece.
        let empty_places = small_spec(json!([{ "name": "A", "do": [
            { "choose": "PLACE", "in": { "region": "BOARD", "where": { "empty": true } } }
        ] }]));
        let game = with_pieces(
            &empty_places,
            &[(pos![0, 1], "GUARD", BLACK), (pos![1, 1], "PAWN", WHITE)],
        );
        let ways = game.compute_ways();
        assert_eq!(ways.len(), 14);
        assert!(!firsts(&ways).contains(&pos![0, 1]));
        assert!(!firsts(&ways).contains(&pos![1, 1]));
    }

    #[test]
    fn board_without_a_filter_is_every_place() {
        let every_place = small_spec(json!([{ "name": "A", "do": [
            { "choose": "PLACE", "in": { "region": "BOARD" } },
            count(json!({ "region": "BOARD" }), None, 2, 2)
        ] }]));
        let game = with_pieces(
            &every_place,
            &[(pos![0, 1], "GUARD", BLACK), (pos![1, 1], "PAWN", WHITE)],
        );
        assert_eq!(
            game.compute_ways().len(),
            16,
            "every place, and the two pieces on the board"
        );
    }

    /// WHITE PAWNs on 0,0, 0,1 and 1,1 (joined along rows and columns) and on 2,2 (only
    /// diagonally next to 1,1), and a BLACK PAWN on 1,0.
    fn a_chain() -> Vec<(Position, &'static str, &'static str)> {
        vec![
            (pos![0, 0], "PAWN", WHITE),
            (pos![0, 1], "PAWN", WHITE),
            (pos![1, 1], "PAWN", WHITE),
            (pos![2, 2], "PAWN", WHITE),
            (pos![1, 0], "PAWN", BLACK),
        ]
    }

    #[test]
    fn a_group_is_the_pieces_joined_through_orthogonal_neighbours_of_the_same_owner() {
        let of_three = pawn_then(json!([count(
            group("START", "orthogonal", None),
            None,
            3,
            3
        )]));
        assert_eq!(
            firsts(&with_pieces(&of_three, &a_chain()).compute_ways()),
            places(&[pos![0, 0], pos![0, 1], pos![1, 1]]),
            "the three joined PAWNs, not the one on 2,2 nor BLACK's"
        );

        let of_four = pawn_then(json!([count(group("START", "adjacent", None), None, 4, 4)]));
        assert_eq!(
            firsts(&with_pieces(&of_four, &a_chain()).compute_ways()),
            places(&[pos![0, 0], pos![0, 1], pos![1, 1], pos![2, 2]]),
            "through adjacent neighbours the PAWN on 2,2 joins"
        );
    }

    #[test]
    fn a_group_is_empty_where_the_place_holds_no_matching_piece() {
        let no_group = |filter: Option<Value>| {
            small_spec(json!([{ "name": "A", "do": [
                { "choose": "PLACE", "in": { "region": "BOARD" } },
                count(group("TARGET", "orthogonal", filter), None, 0, 0)
            ] }]))
        };
        let pieces = [
            (pos![0, 0], "PAWN", WHITE),
            (pos![0, 1], "PROBE", WHITE),
            (pos![3, 3], "PAWN", BLACK),
        ];

        let ways = with_pieces(&no_group(None), &pieces).compute_ways();
        assert_eq!(ways.len(), 13, "the places that hold no piece");
        assert!(!firsts(&ways).contains(&pos![0, 0]));

        let only_probes = no_group(Some(json!({ "codes": ["PROBE"] })));
        let ways = with_pieces(&only_probes, &pieces).compute_ways();
        assert_eq!(ways.len(), 15, "every place but the PROBE's");
        assert!(!firsts(&ways).contains(&pos![0, 1]));
    }

    #[test]
    fn a_group_ignores_pieces_whose_code_the_filter_does_not_take() {
        // A PROBE stands between two PAWNs: through PAWNs only, each PAWN is a group of one.
        let pieces = [
            (pos![0, 0], "PAWN", WHITE),
            (pos![0, 1], "PROBE", WHITE),
            (pos![0, 2], "PAWN", WHITE),
        ];
        let pawns = Some(json!({ "codes": ["PAWN"] }));
        let of_one = pawn_then(json!([count(
            group("START", "orthogonal", pawns),
            None,
            1,
            1
        )]));
        assert_eq!(
            firsts(&with_pieces(&of_one, &pieces).compute_ways()),
            places(&[pos![0, 0], pos![0, 2]])
        );

        let of_three = pawn_then(json!([count(
            group("START", "orthogonal", None),
            None,
            3,
            3
        )]));
        assert_eq!(
            firsts(&with_pieces(&of_three, &pieces).compute_ways()),
            places(&[pos![0, 0], pos![0, 2]]),
            "without a filter the PROBE joins them"
        );
    }

    #[test]
    fn around_a_region_is_the_neighbours_of_its_places_not_in_it_without_duplicates() {
        // The group 0,0 0,1 1,1: 1,0 is next to both 0,0 and 1,1, and is listed once.
        let pieces = [
            (pos![0, 0], "PAWN", WHITE),
            (pos![0, 1], "PAWN", WHITE),
            (pos![1, 1], "PAWN", WHITE),
        ];
        let next_to_the_group = pawn_then(json!([{ "choose": "PLACE", "in": around(
            group("START", "orthogonal", None),
            "orthogonal"
        ) }]));
        let ways = with_pieces(&next_to_the_group, &pieces).compute_ways();
        assert_eq!(
            seconds(&ways),
            places(&[pos![1, 0], pos![0, 2], pos![2, 1], pos![1, 2]])
        );
        assert_eq!(ways.len(), 3 * 4, "four places for each PAWN, none twice");
    }

    /// The liberties of the group at `START`: the empty places orthogonally next to it, counted
    /// between `min` and `max`. The group takes only PAWNs.
    fn liberties(min: u16, max: u16) -> Value {
        count(
            around(
                group("START", "orthogonal", Some(json!({ "codes": ["PAWN"] }))),
                "orthogonal",
            ),
            Some(json!({ "empty": true })),
            min,
            max,
        )
    }

    #[test]
    fn count_with_empty_counts_empty_places() {
        // WHITE's group 0,0 0,1, boxed in by BLACK on 1,0 and 1,1; 0,2 is free or not.
        let boxed_in = [
            (pos![0, 0], "PAWN", WHITE),
            (pos![0, 1], "PAWN", WHITE),
            (pos![1, 0], "PAWN", BLACK),
            (pos![1, 1], "PAWN", BLACK),
            (pos![0, 2], "PAWN", BLACK),
        ];
        let none = pawn_then(json!([liberties(0, 0)]));
        assert_eq!(
            firsts(&with_pieces(&none, &boxed_in).compute_ways()),
            places(&[pos![0, 0], pos![0, 1]])
        );

        let one_free = &boxed_in[..4];
        assert!(with_pieces(&none, one_free).compute_ways().is_empty());
        let one = pawn_then(json!([liberties(1, 1)]));
        assert_eq!(
            firsts(&with_pieces(&one, one_free).compute_ways()),
            places(&[pos![0, 0], pos![0, 1]])
        );

        // `min` defaults to 1, as for any COUNT.
        let some = pawn_then(json!([{
            "test": "COUNT",
            "in": around(group("START", "orthogonal", None), "orthogonal"),
            "where": { "empty": true }
        }]));
        assert!(with_pieces(&some, &boxed_in).compute_ways().is_empty());
        assert_eq!(
            firsts(&with_pieces(&some, one_free).compute_ways()),
            places(&[pos![0, 0], pos![0, 1]])
        );
    }

    #[test]
    fn a_neighbour_holding_a_piece_of_another_code_is_not_a_liberty() {
        // 0,2 holds WHITE's PROBE: not in the group of PAWNs, and not empty.
        let pieces = [
            (pos![0, 0], "PAWN", WHITE),
            (pos![0, 1], "PAWN", WHITE),
            (pos![1, 0], "PAWN", BLACK),
            (pos![1, 1], "PAWN", BLACK),
            (pos![0, 2], "PROBE", WHITE),
        ];
        assert_eq!(
            firsts(&with_pieces(&pawn_then(json!([liberties(0, 0)])), &pieces).compute_ways()),
            places(&[pos![0, 0], pos![0, 1]])
        );
    }

    /// `tests/map_spec`'s map: four points in a square, the paths along its sides, the area `f`
    /// inside them and the link `a`-`c`. The game's action is `A`, choosing an ally PAWN, then
    /// `steps`.
    fn map_game(steps: Value) -> Value {
        let point = |name: &str, at: [f64; 2]| json!({ "name": name, "at": at });
        let path =
            |name: &str, a: &str, b: &str| json!({ "name": name, "kind": "path", "ends": [a, b] });
        let mut all =
            vec![json!({ "choose": "PIECE", "where": { "owner": "ALLY", "codes": ["PAWN"] } })];
        all.extend(steps.as_array().cloned().unwrap_or_default());
        json!({
            "name": "MAP GROUPS",
            "board": {
                "shape": "map",
                "places": [
                    point("a", [0.0, 0.0]),
                    point("b", [2.0, 0.0]),
                    point("c", [2.0, 2.0]),
                    point("d", [0.0, 2.0]),
                    path("ab", "a", "b"),
                    path("bc", "b", "c"),
                    path("cd", "c", "d"),
                    path("da", "d", "a"),
                    { "name": "f", "kind": "area", "around": ["a", "b", "c", "d"] }
                ],
                "links": [["a", "c"]]
            },
            "turns": { "order": ["WHITE", "BLACK"] },
            "players": [
                { "name": "WHITE", "starting_positions": [] },
                { "name": "BLACK", "starting_positions": [] }
            ],
            "pieces": [{ "code": "PAWN", "name": "pawn", "moves": [] }],
            "actions": [{ "name": "A", "do": all }]
        })
    }

    #[test]
    fn a_group_and_around_work_through_a_maps_label() {
        // WHITE on a and b, joined along `next`; BLACK on c. `next` of a is b c d, of b is a c:
        // around the group are c and d, and only d is empty.
        let pieces = [
            (Position::from("a"), "PAWN", WHITE),
            (Position::from("b"), "PAWN", WHITE),
            (Position::from("c"), "PAWN", BLACK),
        ];
        let group_of_two = map_game(json!([count(group("START", "next", None), None, 2, 2)]));
        assert_eq!(
            firsts(&with_pieces(&group_of_two, &pieces).compute_ways()),
            places(&[Position::from("a"), Position::from("b")])
        );

        let one_liberty = map_game(json!([count(
            around(group("START", "next", None), "next"),
            Some(json!({ "empty": true })),
            1,
            1
        )]));
        assert_eq!(
            firsts(&with_pieces(&one_liberty, &pieces).compute_ways()),
            places(&[Position::from("a"), Position::from("b")])
        );

        let around_it = map_game(json!([{ "choose": "PLACE", "in": around(
            group("START", "next", None),
            "next"
        ) }]));
        let ways = with_pieces(&around_it, &pieces).compute_ways();
        assert_eq!(
            seconds(&ways),
            places(&[Position::from("c"), Position::from("d")])
        );

        // The paths a place lies on: `paths` of a is ab da, of b is ab bc.
        let paths = map_game(json!([{ "choose": "PLACE", "in": around(
            group("START", "next", None),
            "paths"
        ) }]));
        assert_eq!(
            seconds(&with_pieces(&paths, &pieces).compute_ways()),
            places(&[
                Position::from("ab"),
                Position::from("da"),
                Position::from("bc")
            ])
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

    #[test]
    fn refusals_of_the_new_regions_and_the_empty_filter() {
        let in_count =
            |filter: Value| move_then(count(json!({ "region": "BOARD" }), Some(filter), 0, 0));
        assert_refused(
            &in_count(json!({ "empty": true, "owner": "ALLY" })),
            &["empty", "owner"],
        );
        assert_refused(
            &in_count(json!({ "empty": true, "codes": ["PAWN"] })),
            &["empty"],
        );
        assert_refused(
            &move_then(count(
                json!({ "region": "BOARD", "where": { "empty": true, "moved": false } }),
                None,
                0,
                0,
            )),
            &["empty"],
        );
        assert_refused(
            &move_then(count(
                group("START", "orthogonal", Some(json!({ "empty": true }))),
                None,
                0,
                0,
            )),
            &["empty", "GROUP"],
        );
        // `empty` is about places: a filter of pieces taken or chosen has no use for it.
        assert_refused(&move_then(removed(json!({ "empty": true }))), &["empty"]);
        assert_refused(
            &small_spec(json!([{ "name": "A", "do": [
                { "choose": "PIECE", "where": { "empty": true } }
            ] }])),
            &["empty"],
        );

        assert_refused(
            &move_then(count(
                json!({ "region": "GROUP", "through": "orthogonal" }),
                None,
                0,
                0,
            )),
            &["`of`"],
        );
        assert_refused(
            &move_then(count(group("START", "diagonal", None), None, 0, 0)),
            &["diagonal"],
        );
        // A grid has no `next`: neither a GROUP nor an AROUND of a region goes through it.
        assert_refused(
            &move_then(count(group("START", "next", None), None, 0, 0)),
            &["next"],
        );
        assert_refused(
            &move_then(count(
                around(group("START", "orthogonal", None), "next"),
                None,
                0,
                0,
            )),
            &["next"],
        );
        // A map has no label `rivers`.
        assert_refused(
            &map_game(json!([count(group("START", "rivers", None), None, 0, 0)])),
            &["rivers"],
        );
        assert_refused(
            &map_game(json!([count(
                around(group("START", "next", None), "rivers"),
                None,
                0,
                0
            )])),
            &["rivers"],
        );
        // The region an AROUND is of is checked as any region.
        assert_refused(
            &move_then(count(
                around(json!({ "region": "AT", "at": at("NOWHERE") }), "orthogonal"),
                None,
                0,
                0,
            )),
            &["NOWHERE"],
        );

        for fine in [
            in_count(json!({ "empty": true })),
            move_then(count(
                json!({ "region": "BOARD", "where": { "empty": true } }),
                None,
                0,
                0,
            )),
            move_then(count(
                around(group("START", "orthogonal", None), "adjacent"),
                Some(json!({ "empty": true })),
                0,
                0,
            )),
            move_then(count(
                json!({ "region": "AROUND", "of": at("TARGET") }),
                None,
                0,
                0,
            )),
            map_game(json!([count(
                around(group("START", "next", None), "areas"),
                None,
                0,
                0
            )])),
        ] {
            assert!(read(&fine).is_ok(), "{:?}", read(&fine).err());
        }
    }

    // ------------------------------------------------------------------
    // ATTACKED keeps the real-threat filter and stops at the first attacker
    // ------------------------------------------------------------------

    /// Whether `ATTACKED in BOARD where ALLY codes by ENEMY` holds for the current player on
    /// the pieces as they stand.
    fn leader_attacked(
        game: &Game,
        players: &[crate::specs::PlayerSpec],
        codes: &[String],
    ) -> bool {
        let spec: TestSpec = serde_json::from_value(json!({
            "test": "ATTACKED",
            "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": codes } },
            "by": "ENEMY"
        }))
        .expect("the test is read");
        let test = Test::build(&spec, players, None, &mut Labels::default());
        let player = game.current_player_ref();
        let nowhere = Position::default();
        judge(
            &test,
            &TestContext {
                game,
                pieces: &game.state.pieces,
                player,
                start: None,
                target: &nowhere,
                each: None,
                removed: &[],
            },
        )
    }

    #[test]
    fn atomic_kings_may_stand_side_by_side_under_attacked() {
        let spec = parse_game_spec("./specs/atomic.json").expect("atomic loads");
        let (players, leader) = (spec.players.clone(), spec.leader.clone());
        let mut game = Game::from_spec(spec);
        game.state.pieces.clear();
        // BLACK's rook looks along the rank at WHITE's king, but taking it would explode
        // BLACK's own king, which stands right next to it.
        put(&mut game, pos![4, 3], "KING", WHITE);
        put(&mut game, pos![4, 4], "KING", BLACK);
        put(&mut game, pos![0, 3], "ROOK", BLACK);
        assert!(
            !leader_attacked(&game, &players, &leader),
            "taking the king would explode the capturer's own king"
        );

        game.state.pieces.remove(&pos![4, 4]);
        put(&mut game, pos![7, 7], "KING", BLACK);
        assert!(
            leader_attacked(&game, &players, &leader),
            "with BLACK's king out of the blast the attack is real"
        );
    }

    /// Plays `way` on a clone of `game`, answering its choices one by one.
    fn play(game: &Game, way: &Way) -> Game {
        let mut next = game.clone();
        next.state.ways = Some(vec![way.clone()]);
        for answer in &way.answers {
            next.transition(GameTransition::Choose(answer.clone()))
                .unwrap_or_else(|error| panic!("{answer:?} of {way:?} was refused: {error:?}"));
        }
        next
    }

    #[test]
    fn attacked_agrees_with_leader_in_check_on_every_catalogue_start_and_after_each_first_move() {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("specs");
        let mut judged = 0;
        for entry in std::fs::read_dir(folder).expect("the specs folder is readable") {
            let path = entry.expect("an entry").path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let name = path.display().to_string();
            let Ok(spec) = parse_game_spec(path.to_str().expect("a UTF-8 path")) else {
                continue;
            };
            if spec.leader.is_empty() {
                continue;
            }
            let (players, leader) = (spec.players.clone(), spec.leader.clone());
            let game = Game::from_spec(spec);
            let mut positions = vec![game.clone()];
            let mut seen = BTreeSet::new();
            for way in game.state.ways.clone().unwrap_or_default() {
                if let Some(Answer::Place(from)) = way.answers.first() {
                    if seen.insert(from.clone()) {
                        positions.push(play(&game, &way));
                    }
                }
            }
            for position in positions {
                assert_eq!(
                    leader_attacked(&position, &players, &leader),
                    position.leader_in_check(),
                    "{name}: ATTACKED and leader_in_check disagree"
                );
                judged += 1;
            }
        }
        assert!(judged > 50, "the catalogue was judged ({judged} positions)");
    }

    // ------------------------------------------------------------------
    // every_action and must
    // ------------------------------------------------------------------

    /// `spec` with the top-level `key` set to `value`.
    fn with(mut spec: Value, key: &str, value: Value) -> Value {
        spec[key] = value;
        spec
    }

    /// The action MOVE: choose an ally piece and one of its moves.
    fn move_action() -> Value {
        json!({ "name": "MOVE", "do": [
            { "choose": "PIECE", "where": { "owner": "ALLY" } },
            { "choose": "MOVE" }
        ] })
    }

    /// The action PLACE: choose a place of the board, put a PROBE there.
    fn place_action() -> Value {
        json!({ "name": "PLACE", "do": [
            { "choose": "PLACE", "in": { "region": "BOARD" } },
            { "effect": "PLACE", "at": at("TARGET"), "code": "PROBE" }
        ] })
    }

    /// No way so far takes an enemy piece.
    fn takes_nothing() -> Value {
        json!({ "test": "NOT", "of": removed(json!({ "owner": "ENEMY" })) })
    }

    fn actions_of(ways: &[Way]) -> BTreeSet<String> {
        ways.iter().map(|way| way.action.clone()).collect()
    }

    fn names(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_string()).collect()
    }

    #[test]
    fn an_every_action_test_removes_ways_in_every_action() {
        let spec = small_spec(json!([move_action(), place_action()]));
        let position = |spec: &Value| {
            let mut game = game_of(spec);
            put(&mut game, pos![0, 0], "PROBE", WHITE);
            put(&mut game, pos![0, 1], "PAWN", BLACK);
            game
        };
        let captures = |game: &Game, action: &str| {
            game.compute_ways()
                .iter()
                .any(|way| way.action == action && way.captures)
        };

        let game = position(&spec);
        assert!(captures(&game, "MOVE") && captures(&game, "PLACE"));

        let game = position(&with(
            spec,
            "every_action",
            json!([{ "do": [takes_nothing()] }]),
        ));
        assert!(!captures(&game, "MOVE"), "the capture by a move is gone");
        assert!(
            !captures(&game, "PLACE"),
            "the capture by a placing is gone"
        );
        assert!(!game.compute_ways().is_empty(), "quiet ways stay");
    }

    #[test]
    fn for_limits_an_entry_to_the_actions_it_names() {
        let spec = with(
            small_spec(json!([move_action(), place_action()])),
            "every_action",
            json!([{ "for": ["PLACE"], "do": [takes_nothing()] }]),
        );
        let mut game = game_of(&spec);
        put(&mut game, pos![0, 0], "PROBE", WHITE);
        put(&mut game, pos![0, 1], "PAWN", BLACK);
        let ways = game.compute_ways();
        assert!(ways.iter().any(|way| way.action == "MOVE" && way.captures));
        assert!(!ways.iter().any(|way| way.action == "PLACE" && way.captures));
    }

    #[test]
    fn entries_run_in_the_order_written_after_the_actions_own_steps() {
        let becomes_a_guard = json!({ "effect": "SET_CODE", "at": at("TARGET"), "code": "GUARD" });
        let is_a_guard = json!({
            "test": "COUNT", "in": { "region": "AT", "at": at("TARGET") },
            "where": { "codes": ["GUARD"] }
        });
        let play = |entries: Value| {
            let spec = with(small_spec(json!([move_action()])), "every_action", entries);
            let mut game = game_of(&spec);
            put(&mut game, pos![0, 0], "PROBE", WHITE);
            game.compute_ways().len()
        };
        // The entry sees where the MOVE left the piece, and the second sees the first's effect.
        assert_eq!(
            play(json!([
                { "do": [becomes_a_guard.clone()] },
                { "do": [is_a_guard.clone()] }
            ])),
            1
        );
        assert_eq!(
            play(json!([{ "do": [is_a_guard.clone()] }, { "do": [becomes_a_guard] }])),
            0,
            "the test came first, so it saw a PROBE"
        );
    }

    #[test]
    fn every_action_steps_run_after_a_moves_own_steps() {
        let mut spec = small_spec(json!([move_action()]));
        spec["pieces"][0]["moves"][0]["do"] = json!([
            not_ally(),
            move_effect(),
            { "choose": "CODE", "as": "promo", "options": ["GUARD", "KING"] },
            { "effect": "SET_CODE", "at": at("TARGET"), "code": { "chosen": "promo" } }
        ]);
        let spec = with(
            spec,
            "every_action",
            json!([{ "do": [{
                "test": "COUNT", "in": { "region": "AT", "at": at("TARGET") },
                "where": { "codes": ["KING"] }
            }] }]),
        );
        let mut game = game_of(&spec);
        put(&mut game, pos![0, 0], "PROBE", WHITE);
        let ways = game.compute_ways();
        assert_eq!(
            ways.len(),
            1,
            "only the promotion to a KING passes: {ways:?}"
        );
        assert!(ways[0].answers.contains(&Answer::Code("KING".to_string())));
    }

    #[test]
    fn must_keeps_only_the_ways_that_pass_when_some_way_does() {
        let spec = with(
            small_spec(json!([move_action()])),
            "must",
            json!([removed(json!({ "owner": "ENEMY" }))]),
        );
        let game = a_capture_and_a_quiet_move(&spec);
        let ways = game.compute_ways();
        assert_eq!(firsts(&ways), places(&[pos![0, 0]]));
        assert!(ways.iter().all(|way| way.captures));
    }

    #[test]
    fn must_changes_nothing_when_no_way_passes() {
        let spec = with(
            small_spec(json!([move_action()])),
            "must",
            json!([removed(json!({ "owner": "ENEMY" }))]),
        );
        let mut game = game_of(&spec);
        put(&mut game, pos![0, 0], "PROBE", WHITE);
        put(&mut game, pos![2, 0], "PROBE", WHITE);
        assert_eq!(
            firsts(&game.compute_ways()),
            places(&[pos![0, 0], pos![2, 0]])
        );
    }

    #[test]
    fn must_entries_narrow_in_order() {
        let takes = |code: &str| removed(json!({ "owner": "ENEMY", "codes": [code] }));
        let position = |entries: Value| {
            let spec = with(small_spec(json!([move_action()])), "must", entries);
            let mut game = game_of(&spec);
            put(&mut game, pos![0, 0], "PROBE", WHITE);
            put(&mut game, pos![0, 1], "PAWN", BLACK);
            put(&mut game, pos![2, 3], "GUARD", WHITE);
            put(&mut game, pos![1, 3], "GUARD", BLACK);
            game.compute_ways()
        };
        // No way takes both, so the entry written first decides; the second finds nothing and
        // changes nothing.
        assert_eq!(
            firsts(&position(json!([takes("PAWN"), takes("GUARD")]))),
            places(&[pos![0, 0]])
        );
        assert_eq!(
            firsts(&position(json!([takes("GUARD"), takes("PAWN")]))),
            places(&[pos![2, 3]])
        );
    }

    #[test]
    fn must_removes_drops_and_passes_while_a_capture_exists() {
        let drop = json!({ "name": "DROP", "do": [
            { "choose": "PLACE", "in": { "region": "BOARD" } },
            { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": at("TARGET") } } },
            { "effect": "PLACE", "at": at("TARGET"), "code": "PROBE" }
        ] });
        let spec = with(
            small_spec(json!([move_action(), drop, { "name": "PASS", "do": [] }])),
            "must",
            json!([removed(json!({ "owner": "ENEMY" }))]),
        );
        let game = a_capture_and_a_quiet_move(&spec);
        assert_eq!(actions_of(&game.compute_ways()), names(&["MOVE"]));

        let mut quiet = game_of(&spec);
        put(&mut quiet, pos![2, 0], "PROBE", WHITE);
        assert_eq!(
            actions_of(&quiet.compute_ways()),
            names(&["MOVE", "DROP", "PASS"]),
            "with no capture to make, everything is offered"
        );
    }

    #[test]
    fn every_action_and_must_are_refused_when_wrong() {
        let actions = || small_spec(json!([move_action()]));
        let with_entry = |entry: Value| with(actions(), "every_action", json!([entry]));

        assert_refused(
            &with_entry(json!({ "for": ["JUMP"], "do": [] })),
            &["JUMP", "every_action"],
        );
        assert_refused(&with_entry(json!({ "for": [], "do": [] })), &["for"]);
        assert_refused(
            &with_entry(json!({ "do": [{ "choose": "PLACE", "in": { "region": "BOARD" } }] })),
            &["choose", "every_action"],
        );
        assert_refused(
            &with_entry(json!({ "do": [{ "if": takes_nothing(), "then": [
                { "choose": "PLACE", "in": { "region": "BOARD" } }
            ] }] })),
            &["choose", "every_action"],
        );
        assert_refused(&with_entry(json!({ "do": [], "extra": 1 })), &["extra"]);
        assert_refused(&with_entry(json!({ "for": ["MOVE"] })), &["do"]);
        // A place the action has not set is refused where it is used.
        assert_refused(
            &with(
                small_spec(json!([{ "name": "PASS", "do": [] }])),
                "every_action",
                json!([{ "do": [{ "effect": "REMOVE", "at": at("TARGET") }] }]),
            ),
            &["TARGET", "PASS"],
        );
        assert_refused(
            &with(actions(), "must", json!([{ "effect": "END_TURN" }])),
            &[],
        );
        assert_refused(
            &with(
                actions(),
                "must",
                json!([{ "test": "COUNT", "in": {
                "region": "AT", "at": at("TARGET") } }]),
            ),
            &["TARGET", "must"],
        );
        for key in ["every_action", "must"] {
            let repeats = if key == "must" {
                json!([{ "test": "REPEATS" }])
            } else {
                json!([{ "do": [{ "test": "REPEATS" }] }])
            };
            let spec = with(with(actions(), "jump_enabled", json!(true)), key, repeats);
            assert_refused(&spec, &["REPEATS", "jump_enabled"]);
        }
        for fine in [
            with_entry(json!({ "do": [takes_nothing()] })),
            with_entry(json!({ "for": ["MOVE"], "do": [] })),
            with(
                actions(),
                "must",
                json!([removed(json!({ "owner": "ENEMY" }))]),
            ),
        ] {
            assert!(read(&fine).is_ok(), "{:?}", read(&fine).err());
        }
    }

    #[test]
    fn a_for_each_region_that_reads_pieces_sees_the_position_the_steps_so_far_left() {
        // The PROBE put on TARGET is in the GROUP of TARGET, so the FOR_EACH finds it and
        // removes it, and the closing test sees an ally piece removed. On the position the
        // action started from, TARGET is empty (or holds the PAWN) and the group is too.
        let spec = place_then(json!([
            { "effect": "PLACE", "at": at("TARGET"), "code": "PROBE" },
            { "effect": "FOR_EACH",
              "in": { "region": "GROUP", "of": at("TARGET"), "through": "orthogonal" },
              "do": [{ "effect": "REMOVE", "at": at("EACH") }] },
            removed(json!({ "owner": "ALLY", "codes": ["PROBE"] }))
        ]));
        let mut game = game_of(&spec);
        put(&mut game, pos![0, 0], "PAWN", BLACK);
        let ways = game.compute_ways();
        assert_eq!(ways.len(), 16, "every place, the PAWN's too");
    }

    #[test]
    fn a_choice_region_that_reads_pieces_sees_the_position_the_steps_so_far_left() {
        // After the PAWN steps away, the board holds a single piece, and a PLACE choice over
        // the places holding a piece offers only where it went.
        let spec = small_spec(json!([{ "name": "A", "do": [
            { "choose": "PIECE", "where": { "owner": "ALLY", "codes": ["PAWN"] } },
            { "choose": "PLACE", "in": { "region": "BOARD" } },
            { "test": "NOT", "of": { "test": "COUNT", "in": { "region": "AT", "at": at("TARGET") } } },
            move_effect(),
            { "choose": "PLACE", "in": { "region": "BOARD", "where": { "owner": "ALLY" } } }
        ] }]));
        let mut game = game_of(&spec);
        put(&mut game, pos![0, 0], "PAWN", WHITE);
        let ways = game.compute_ways();
        assert!(!ways.is_empty());
        for way in &ways {
            assert_eq!(
                way.answers[2], way.answers[1],
                "the second place is where the PAWN now stands"
            );
        }
    }

    #[test]
    fn must_entries_after_the_first_judge_each_way_on_its_own_position() {
        // Entry one keeps the two capturing ways. Entry two reads the pieces: it holds when
        // exactly one enemy PAWN is left, which is true after either capture and false after
        // the quiet move that came first. Each way must be judged on its own position.
        let spec = with(
            small_spec(json!([move_action()])),
            "must",
            json!([
                removed(json!({ "owner": "ENEMY" })),
                { "test": "COUNT", "in": { "region": "BOARD" },
                  "where": { "owner": "ENEMY", "codes": ["PAWN"] }, "min": 1, "max": 1 }
            ]),
        );
        let mut game = game_of(&spec);
        put(&mut game, pos![0, 0], "PROBE", WHITE); // a quiet step to 0,1
        put(&mut game, pos![2, 0], "PROBE", WHITE); // takes the PAWN on 2,1
        put(&mut game, pos![3, 0], "PROBE", WHITE); // takes the PAWN on 3,1
        put(&mut game, pos![2, 1], "PAWN", BLACK);
        put(&mut game, pos![3, 1], "PAWN", BLACK);
        let ways = game.compute_ways();
        assert_eq!(firsts(&ways), places(&[pos![2, 0], pos![3, 0]]));
    }
}
