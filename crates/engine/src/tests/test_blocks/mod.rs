//! Tests as blocks in a move's and an action's conditions: each test, region, place and filter
//! kind, the empty cases of the logic tests, and every refusal when the spec is read.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Game, GameTransition, Piece, PieceState};
    use crate::shared::Position;
    use crate::specs::{parse_game_spec, parse_game_spec_str, GameSpec, GameSpecError};

    const WHITE: &str = "WHITE";
    const BLACK: &str = "BLACK";
    const SIDE: &str = "SIDE";

    fn load_game() -> Game {
        parse_game_spec("./src/tests/test_blocks/spec.json")
            .map(Game::from_spec)
            .expect("Failed to load test_blocks test spec")
    }

    fn insert(game: &mut Game, pos: [u8; 2], code: &str, player: &str) {
        game.state.pieces.insert(
            Position::from_coordinates(&pos),
            Piece::new(code.to_string(), player.to_string()),
        );
    }

    fn piece_mut(game: &mut Game, pos: [u8; 2]) -> &mut Piece {
        game.state
            .pieces
            .get_mut(&Position::from_coordinates(&pos))
            .expect("a piece stands there")
    }

    /// Whether the piece at `from` is offered `to`, for the current player.
    fn can_move(game: &mut Game, from: [u8; 2], to: [u8; 2]) -> bool {
        game.transition(GameTransition::CalculateMoves {
            position: Position::from_coordinates(&from),
        })
        .expect("selecting the piece should succeed");
        game.state
            .available_moves
            .as_ref()
            .is_some_and(|m| m.contains_key(&Position::from_coordinates(&to)))
    }

    /// A game where `code` of `player` stands on `at`, and that player is to move.
    fn game_with(code: &str, at: [u8; 2], player: &str) -> Game {
        let mut game = load_game();
        game.state.current_turn = match player {
            WHITE => 0,
            BLACK => 1,
            _ => 2,
        };
        insert(&mut game, at, code, player);
        game
    }

    #[test]
    fn count_at_an_offset_sees_an_enemy() {
        let mut game = game_with("SEER", [2, 2], WHITE);
        assert!(!can_move(&mut game, [2, 2], [2, 3]), "nobody at [3,3]");

        insert(&mut game, [3, 3], "BLOCK", WHITE);
        assert!(!can_move(&mut game, [2, 2], [2, 3]), "an ally at [3,3]");

        insert(&mut game, [3, 3], "BLOCK", BLACK);
        assert!(can_move(&mut game, [2, 2], [2, 3]), "an enemy at [3,3]");
    }

    #[test]
    fn count_with_codes_flag_and_moved_filters() {
        // Codes: any of ROOK and CANNON on [4,3].
        let mut game = game_with("PICKY", [3, 3], WHITE);
        insert(&mut game, [4, 3], "BLOCK", BLACK);
        assert!(
            !can_move(&mut game, [3, 3], [3, 4]),
            "a BLOCK is no ROOK or CANNON"
        );
        insert(&mut game, [4, 3], "CANNON", BLACK);
        assert!(
            can_move(&mut game, [3, 3], [3, 4]),
            "a CANNON is one of the codes"
        );

        // Flag: EN_PASSANT on [2,3].
        let mut game = game_with("PICKY", [3, 3], WHITE);
        insert(&mut game, [2, 3], "BLOCK", BLACK);
        assert!(!can_move(&mut game, [3, 3], [3, 2]), "no flag");
        piece_mut(&mut game, [2, 3])
            .state
            .insert("EN_PASSANT".to_string(), PieceState::Blank);
        assert!(can_move(&mut game, [3, 3], [3, 2]), "the flag is set");

        // Moved false: the piece on [4,4] never moved.
        let mut game = game_with("PICKY", [3, 3], WHITE);
        insert(&mut game, [4, 4], "BLOCK", BLACK);
        assert!(can_move(&mut game, [3, 3], [4, 3]), "never moved");
        piece_mut(&mut game, [4, 4]).total_moves = 1;
        assert!(!can_move(&mut game, [3, 3], [4, 3]), "it has moved");

        // Moved true: the piece on [2,2] has moved.
        let mut game = game_with("PICKY", [3, 3], WHITE);
        insert(&mut game, [2, 2], "BLOCK", BLACK);
        assert!(!can_move(&mut game, [3, 3], [2, 3]), "never moved");
        piece_mut(&mut game, [2, 2]).total_moves = 2;
        assert!(can_move(&mut game, [3, 3], [2, 3]), "it has moved");
    }

    #[test]
    fn count_bounds_min_and_max() {
        // PATHER: min 0, max 0 over the path from [1,1] to [1,4].
        let mut game = game_with("PATHER", [1, 1], WHITE);
        assert!(can_move(&mut game, [1, 1], [1, 4]), "an empty path");
        insert(&mut game, [1, 2], "BLOCK", BLACK);
        assert!(
            !can_move(&mut game, [1, 1], [1, 4]),
            "one piece on the path"
        );

        // VAULTER: min 1, max 1.
        let mut game = game_with("VAULTER", [1, 1], WHITE);
        assert!(!can_move(&mut game, [1, 1], [1, 4]), "an empty path");
        insert(&mut game, [1, 2], "BLOCK", BLACK);
        assert!(can_move(&mut game, [1, 1], [1, 4]), "exactly one piece");
        insert(&mut game, [1, 3], "BLOCK", WHITE);
        assert!(!can_move(&mut game, [1, 1], [1, 4]), "two pieces");
    }

    #[test]
    fn count_at_target_is_judged_on_every_square_of_a_slide() {
        let mut game = game_with("SLIDER", [0, 0], WHITE);
        insert(&mut game, [0, 2], "CANNON", BLACK);
        insert(&mut game, [0, 4], "BLOCK", BLACK);
        insert(&mut game, [0, 6], "CANNON", BLACK);

        let offered: Vec<bool> = (1..8u8)
            .map(|y| can_move(&mut game, [0, 0], [0, y]))
            .collect();
        assert_eq!(
            offered,
            vec![true, false, true, true, true, false, true],
            "every square but the cannons' is offered"
        );
    }

    #[test]
    fn path_counts_between_start_and_target_without_ends() {
        let mut game = game_with("PATHER", [1, 1], WHITE);
        insert(&mut game, [1, 4], "BLOCK", BLACK);
        assert!(
            can_move(&mut game, [1, 1], [1, 4]),
            "neither the mover on FROM nor the enemy on TO is counted"
        );
        insert(&mut game, [1, 3], "BLOCK", BLACK);
        assert!(!can_move(&mut game, [1, 1], [1, 4]), "a piece between");
    }

    #[test]
    fn path_ends_include_from_and_to() {
        // ENDER: at least 1 and at most 1 piece on the path with both ends.
        let mut game = game_with("ENDER", [1, 1], WHITE);
        assert!(
            can_move(&mut game, [1, 1], [1, 4]),
            "the mover on FROM is the one piece"
        );
        insert(&mut game, [1, 4], "BLOCK", BLACK);
        assert!(
            !can_move(&mut game, [1, 1], [1, 4]),
            "the enemy on TO makes two"
        );
    }

    #[test]
    fn count_over_a_broken_path_fails() {
        // [6,4] is disabled: the path from [6,2] to [6,5] crosses it.
        let mut game = game_with("PATHER", [6, 2], WHITE);
        insert(&mut game, [5, 2], "PATHER", WHITE);
        assert!(
            can_move(&mut game, [5, 2], [5, 5]),
            "the next file is whole"
        );
        assert!(
            !can_move(&mut game, [6, 2], [6, 5]),
            "the path crosses a hole"
        );

        // A step of [2, 1] is no straight line.
        let mut game = game_with("SKEW", [2, 2], WHITE);
        assert!(
            can_move(&mut game, [2, 2], [3, 4]),
            "the unconditioned step"
        );
        assert!(!can_move(&mut game, [2, 2], [4, 3]), "no path along [2, 1]");
    }

    #[test]
    fn not_over_an_off_board_place_holds() {
        let mut game = game_with("EDGE", [3, 7], WHITE);
        assert!(
            can_move(&mut game, [3, 7], [4, 7]),
            "[3,8] is off the board"
        );

        let mut game = game_with("EDGE", [3, 5], WHITE);
        assert!(can_move(&mut game, [3, 5], [4, 5]), "[3,6] is empty");
        insert(&mut game, [3, 6], "BLOCK", BLACK);
        assert!(!can_move(&mut game, [3, 5], [4, 5]), "[3,6] is taken");
    }

    #[test]
    fn around_counts_adjacent_and_orthogonal_neighbours() {
        let mut game = game_with("CROWD", [3, 3], WHITE);
        insert(&mut game, [2, 2], "BLOCK", WHITE);
        insert(&mut game, [4, 4], "BLOCK", WHITE);
        insert(&mut game, [2, 3], "BLOCK", BLACK);
        assert!(
            can_move(&mut game, [3, 3], [3, 4]),
            "two allies diagonally adjacent"
        );
        assert!(
            !can_move(&mut game, [3, 3], [3, 2]),
            "no ally orthogonally next to it"
        );

        insert(&mut game, [4, 3], "BLOCK", WHITE);
        assert!(
            !can_move(&mut game, [3, 3], [3, 2]),
            "one ally orthogonally next to it"
        );
        insert(&mut game, [2, 3], "BLOCK", WHITE);
        assert!(
            can_move(&mut game, [3, 3], [3, 2]),
            "two allies orthogonally next to it"
        );
    }

    #[test]
    fn attacked_sees_an_enemy_attack_on_the_region() {
        let mut game = game_with("WARY", [3, 3], WHITE);
        insert(&mut game, [7, 0], "ROOK", BLACK);
        assert!(
            can_move(&mut game, [3, 3], [3, 4]),
            "rank 4 is not attacked"
        );

        insert(&mut game, [0, 4], "ROOK", WHITE);
        assert!(
            can_move(&mut game, [3, 3], [3, 4]),
            "an ally's attack does not count"
        );

        insert(&mut game, [0, 4], "ROOK", BLACK);
        assert!(
            !can_move(&mut game, [3, 3], [3, 4]),
            "an enemy rook attacks [3,4]"
        );
    }

    #[test]
    fn in_zone_reads_the_movers_part_of_a_zone() {
        let mut game = game_with("GUARD", [3, 0], WHITE);
        assert!(
            can_move(&mut game, [3, 0], [3, 1]),
            "[3,1] is in WHITE's palace"
        );

        let mut game = game_with("GUARD", [3, 1], WHITE);
        assert!(!can_move(&mut game, [3, 1], [3, 2]), "[3,2] is outside it");

        let mut game = game_with("GUARD", [3, 5], WHITE);
        assert!(
            !can_move(&mut game, [3, 5], [3, 6]),
            "[3,6] is in BLACK's palace, not WHITE's"
        );

        let mut game = game_with("GUARD", [3, 7], BLACK);
        assert!(
            can_move(&mut game, [3, 7], [3, 6]),
            "[3,6] is in BLACK's palace"
        );
    }

    #[test]
    fn line_counts_both_ways_through_the_place() {
        let mut game = game_with("FILER", [2, 3], WHITE);
        insert(&mut game, [4, 0], "BLOCK", BLACK);
        insert(&mut game, [5, 0], "BLOCK", WHITE);
        assert!(
            can_move(&mut game, [2, 3], [3, 3]),
            "an enemy on the file, an ally on another"
        );

        insert(&mut game, [3, 7], "BLOCK", WHITE);
        assert!(
            !can_move(&mut game, [2, 3], [3, 3]),
            "an ally ahead on the file"
        );

        let mut game = game_with("FILER", [2, 3], WHITE);
        insert(&mut game, [3, 0], "BLOCK", WHITE);
        assert!(
            !can_move(&mut game, [2, 3], [3, 3]),
            "an ally behind on the file"
        );
    }

    #[test]
    fn all_any_not_and_their_empty_cases() {
        let mut game = game_with("LOGIC", [3, 3], WHITE);
        assert!(can_move(&mut game, [3, 3], [3, 4]), "ALL [] holds");
        assert!(!can_move(&mut game, [3, 3], [4, 3]), "ANY [] fails");
        assert!(can_move(&mut game, [3, 3], [2, 3]), "NOT ANY [] holds");
        assert!(!can_move(&mut game, [3, 3], [3, 2]), "NOT ALL [] fails");
        assert!(
            can_move(&mut game, [3, 3], [4, 4]),
            "ANY [fails, holds] holds"
        );
        assert!(
            !can_move(&mut game, [3, 3], [2, 2]),
            "ALL [holds, fails] fails"
        );
    }

    #[test]
    fn an_offset_is_turned_for_a_sideways_player() {
        // SIDE turns [0, 1] into [-1, 0] and [1, 1] into [-1, 1].
        let mut game = game_with("SEER", [4, 4], SIDE);
        insert(&mut game, [5, 5], "BLOCK", WHITE);
        assert!(
            !can_move(&mut game, [4, 4], [3, 4]),
            "the offset as written"
        );

        let mut game = game_with("SEER", [4, 4], SIDE);
        insert(&mut game, [3, 5], "BLOCK", WHITE);
        assert!(
            can_move(&mut game, [4, 4], [3, 4]),
            "the offset turned for SIDE"
        );
    }

    // ---------------------------------------------------------------------
    // Refusals when the spec is read
    // ---------------------------------------------------------------------

    /// A game on a 4×4 grid, with a zone PALACE, whose PROBE steps [0, 1] under `conditions`.
    fn grid_spec(conditions: Value) -> String {
        json!({
            "name": "BLOCKS",
            "board": { "dimensions": [4, 4] },
            "conditions": [
                { "code": "PALACE", "type": "POSITION", "check": { "WHITE": [[0, 0]], "BLACK": [[3, 3]] } }
            ],
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "pieces": [
                {
                    "code": "PROBE",
                    "name": "probe",
                    "moves": [{
                        "id": 0,
                        "step": [0, 1],
                        "actions": [{ "state": "EMPTY", "action": "MOVE" }],
                        "conditions": conditions
                    }]
                }
            ]
        })
        .to_string()
    }

    /// A game on a map of three points in a row, whose PROBE steps along `next` under
    /// `conditions`.
    fn map_spec(conditions: Value) -> String {
        json!({
            "name": "BLOCKS ON A MAP",
            "board": {
                "shape": "map",
                "places": [
                    { "name": "a", "at": [0.0, 0.0] },
                    { "name": "b", "at": [1.0, 0.0] },
                    { "name": "c", "at": [2.0, 0.0] }
                ],
                "links": [["a", "b"], ["b", "c"]]
            },
            "leader": [],
            "players": [
                { "name": "A", "starting_positions": [] },
                { "name": "B", "starting_positions": [] }
            ],
            "turns": { "order": ["A", "B"] },
            "pieces": [
                {
                    "code": "PROBE",
                    "name": "probe",
                    "moves": [{
                        "id": 0,
                        "step": "next",
                        "actions": [{ "state": "EMPTY", "action": "MOVE" }],
                        "conditions": conditions
                    }]
                }
            ]
        })
        .to_string()
    }

    fn read(spec: String) -> Result<GameSpec, GameSpecError> {
        parse_game_spec_str(&spec)
    }

    /// `COUNT` of any piece at `at`.
    fn count_at(at: Value) -> Value {
        json!({ "test": "COUNT", "in": { "region": "AT", "at": at } })
    }

    #[test]
    fn a_well_written_block_is_read() {
        let spec = grid_spec(json!([count_at(
            json!({ "place": "START", "offset": [0, 1] })
        )]));
        assert!(read(spec).is_ok());
    }

    #[test]
    fn a_misspelled_field_in_a_block_is_refused() {
        let wher = json!([{
            "test": "COUNT",
            "in": { "region": "AT", "at": { "place": "START" } },
            "wher": { "owner": "ENEMY" }
        }]);
        let offest = json!([count_at(json!({ "place": "START", "offest": [0, 1] }))]);
        for (field, conditions) in [("wher", wher), ("offest", offest)] {
            let error = read(grid_spec(conditions))
                .err()
                .unwrap_or_else(|| panic!("`{field}` should be refused"));
            assert!(
                error
                    .to_string()
                    .contains(&format!("unknown field `{field}`")),
                "the refusal names `{field}`: {error}"
            );
        }
    }

    #[test]
    fn refusals_when_the_spec_is_read() {
        let cases: Vec<(&str, Value)> = vec![
            (
                "KREMLIN",
                json!({ "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "KREMLIN" }),
            ),
            (
                "DRAGON",
                json!({
                    "test": "COUNT",
                    "in": { "region": "AT", "at": { "place": "TARGET" } },
                    "where": { "codes": ["DRAGON"] }
                }),
            ),
            (
                "min is 3",
                json!({
                    "test": "COUNT",
                    "in": { "region": "AT", "at": { "place": "TARGET" } },
                    "min": 3,
                    "max": 1
                }),
            ),
            (
                "paths",
                json!({
                    "test": "COUNT",
                    "in": { "region": "AROUND", "of": { "place": "START" }, "through": "paths" }
                }),
            ),
            (
                "FRIEND",
                json!({
                    "test": "COUNT",
                    "in": { "region": "AT", "at": { "place": "TARGET" } },
                    "where": { "owner": "FRIEND" }
                }),
            ),
            (
                "ALLY",
                json!({
                    "test": "ATTACKED",
                    "in": { "region": "AT", "at": { "place": "TARGET" } },
                    "by": "ALLY"
                }),
            ),
            (
                "MIDDLE",
                json!({
                    "test": "COUNT",
                    "in": {
                        "region": "PATH",
                        "from": { "place": "START" },
                        "to": { "place": "TARGET" },
                        "ends": ["MIDDLE"]
                    }
                }),
            ),
            ("HERE", count_at(json!({ "place": "HERE" }))),
        ];
        for (named, test) in cases {
            let error = read(grid_spec(json!([test])))
                .err()
                .unwrap_or_else(|| panic!("the spec naming {named} should be refused"));
            assert!(
                !matches!(error, GameSpecError::JsonError(_)),
                "{named}: refused while checking, not while parsing: {error}"
            );
            assert!(
                error.to_string().contains(named),
                "the refusal names {named}: {error}"
            );
        }
    }

    #[test]
    fn refusals_reach_into_nested_tests_and_action_conditions() {
        let nested = json!([{
            "test": "NOT",
            "of": { "test": "ALL", "of": [count_at(json!({ "place": "HERE" }))] }
        }]);
        assert!(
            read(grid_spec(nested)).is_err(),
            "a place inside NOT and ALL"
        );

        let mut spec: Value =
            serde_json::from_str(&grid_spec(json!([]))).expect("the spec is JSON");
        spec["pieces"][0]["moves"][0]["actions"][0]["conditions"] =
            json!([count_at(json!({ "place": "HERE" }))]);
        assert!(
            read(spec.to_string()).is_err(),
            "a place in an action's conditions"
        );
    }

    #[test]
    fn grid_only_blocks_are_refused_on_a_map() {
        assert!(
            read(map_spec(json!([count_at(json!({ "place": "TARGET" }))]))).is_ok(),
            "a place without an offset is fine on a map"
        );
        assert!(
            read(map_spec(json!([{
                "test": "COUNT",
                "in": { "region": "AROUND", "of": { "place": "TARGET" }, "through": "paths" }
            }])))
            .is_ok(),
            "AROUND through a map's label is fine on a map"
        );

        let cases = vec![
            (
                "an offset",
                count_at(json!({ "place": "START", "offset": [0, 1] })),
            ),
            (
                "a PATH",
                json!({
                    "test": "COUNT",
                    "in": { "region": "PATH", "from": { "place": "START" }, "to": { "place": "TARGET" } }
                }),
            ),
            (
                "a LINE",
                json!({
                    "test": "COUNT",
                    "in": { "region": "LINE", "through": { "place": "TARGET" }, "along": [0, 1] }
                }),
            ),
        ];
        for (what, test) in cases {
            let result = read(map_spec(json!([test])));
            assert!(
                matches!(result, Err(GameSpecError::NotOnAMap(_))),
                "{what} on a map: {:?}",
                result.err()
            );
        }
    }
}
