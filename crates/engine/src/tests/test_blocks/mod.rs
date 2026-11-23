//! Tests as blocks in the steps of a move and an action: each test, region, place and filter
//! kind, the empty cases of the logic tests, and every refusal when the spec is read.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Game, GamePhase, Piece, PieceState};
    use crate::shared::Position;
    use crate::specs::{parse_game_spec, parse_game_spec_str, GameSpec, GameSpecError};
    use crate::tests::support::{drop_squares, go, pick, targets, way_to};

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

    /// `TARGET` holds no piece.
    fn target_empty() -> Value {
        json!({
            "test": "NOT",
            "of": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } } }
        })
    }

    /// The piece at `START` goes to `TARGET`.
    fn step_to_target() -> Value {
        json!({ "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } })
    }

    /// A move's steps: `TARGET` is empty, every test of `conditions` holds, the piece goes there.
    fn steps(conditions: Value) -> Value {
        let mut steps = vec![target_empty()];
        steps.extend(
            conditions
                .as_array()
                .cloned()
                .expect("conditions are a list"),
        );
        steps.push(step_to_target());
        Value::Array(steps)
    }

    /// The action MOVE: pick a piece of the mover's and one of its moves.
    fn move_action() -> Value {
        json!({ "name": "MOVE", "do": [
            { "choose": "PIECE", "where": { "owner": "ALLY" } },
            { "choose": "MOVE" }
        ] })
    }

    fn piece_mut(game: &mut Game, pos: [u8; 2]) -> &mut Piece {
        game.state
            .pieces
            .get_mut(&Position::from_coordinates(&pos))
            .expect("a piece stands there")
    }

    /// Whether the piece at `from` is offered `to`, for the current player.
    fn can_move(game: &mut Game, from: [u8; 2], to: [u8; 2]) -> bool {
        pick(game, Position::from_coordinates(&from)).expect("selecting the piece should succeed");
        targets(game).contains(&Position::from_coordinates(&to))
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
            .insert(crate::shared::Symbol::new("EN_PASSANT"), PieceState::Blank);
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

        // A step of [2, 1] is no straight line: reading the spec refuses the PATH along it,
        // and a game built without that check never forms it.
        let skew = skew_spec();
        let refused = read(skew.to_string()).expect_err("the PATH along [2, 1] is refused");
        assert!(
            matches!(&refused, GameSpecError::InvalidTest(why) if why.contains("move 0 of piece SKEW")),
            "{refused}"
        );
        let unchecked: GameSpec = serde_json::from_value(skew).expect("the spec is written well");
        let mut game = Game::from_spec(unchecked);
        game.state.current_turn = 0;
        insert(&mut game, [2, 2], "SKEW", WHITE);
        assert!(
            can_move(&mut game, [2, 2], [3, 4]),
            "the unconditioned step"
        );
        assert!(!can_move(&mut game, [2, 2], [4, 3]), "no path along [2, 1]");
    }

    /// The test spec with SKEW, whose move 0 counts the PATH along its [2, 1] step (no straight
    /// line) and whose move 1 steps [1, 2] unconditioned.
    fn skew_spec() -> Value {
        let mut spec: Value = serde_json::from_str(
            &std::fs::read_to_string("./src/tests/test_blocks/spec.json").expect("the test spec"),
        )
        .expect("the test spec is JSON");
        spec["pieces"]
            .as_array_mut()
            .expect("the spec has pieces")
            .push(json!({
                "code": "SKEW",
                "name": "skew",
                "moves": [
                    {
                        "id": 0,
                        "step": [2, 1],
                        "do": steps(json!([{
                            "test": "COUNT",
                            "in": {
                                "region": "PATH",
                                "from": { "place": "START" },
                                "to": { "place": "TARGET" }
                            },
                            "min": 0,
                            "max": 0
                        }]))
                    },
                    {
                        "id": 1,
                        "step": [1, 2],
                        "do": steps(json!([]))
                    }
                ]
            }));
        spec
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

    #[test]
    fn line_along_is_turned_for_each_player() {
        // FILER looks along [0, 1] through its landing square: a file for WHITE, a rank for SIDE.
        let mut game = game_with("FILER", [2, 3], WHITE);
        insert(&mut game, [3, 6], "BLOCK", WHITE);
        assert!(
            !can_move(&mut game, [2, 3], [3, 3]),
            "WHITE: an ally on the landing square's file"
        );

        // SIDE steps [1, 0] as [0, 1], to [2, 4].
        let mut game = game_with("FILER", [2, 3], SIDE);
        insert(&mut game, [2, 7], "BLOCK", SIDE);
        assert!(
            can_move(&mut game, [2, 3], [2, 4]),
            "SIDE: an ally on the landing square's file is not on its line"
        );
        insert(&mut game, [5, 4], "BLOCK", SIDE);
        assert!(
            !can_move(&mut game, [2, 3], [2, 4]),
            "SIDE: an ally on the landing square's rank"
        );
    }

    /// The code of the piece standing on `to` once the move from `from` to `to` is played, if
    /// it is offered.
    fn code_after(game: &mut Game, from: [u8; 2], to: [u8; 2]) -> Option<String> {
        pick(game, Position::from_coordinates(&from)).expect("selecting the piece should succeed");
        let to = Position::from_coordinates(&to);
        let way = way_to(game, to)?;
        way.changes
            .iter()
            .rev()
            .find(|change| change.position == to)
            .and_then(|change| change.piece.as_ref())
            .map(|piece| piece.code.to_string())
    }

    #[test]
    fn a_promotion_takes_any_test() {
        // PROMOTER becomes a ROOK when an ally stands behind its square.
        let mut game = game_with("PROMOTER", [3, 3], WHITE);
        assert_eq!(
            code_after(&mut game, [3, 3], [3, 4]).as_deref(),
            Some("PROMOTER"),
            "nobody behind"
        );
        insert(&mut game, [3, 2], "BLOCK", BLACK);
        assert_eq!(
            code_after(&mut game, [3, 3], [3, 4]).as_deref(),
            Some("PROMOTER"),
            "an enemy behind"
        );
        insert(&mut game, [3, 2], "BLOCK", WHITE);
        assert_eq!(
            code_after(&mut game, [3, 3], [3, 4]).as_deref(),
            Some("ROOK"),
            "an ally behind"
        );
    }

    #[test]
    fn repeat_until_takes_a_test() {
        // ENDLESS never stops early: what it is offered on each square is up to its actions.
        let mut game = game_with("ENDLESS", [1, 0], WHITE);
        let offered: Vec<bool> = (1..8u8)
            .map(|y| can_move(&mut game, [1, 0], [1, y]))
            .collect();
        assert_eq!(offered, vec![true; 7], "to the edge over empty squares");

        insert(&mut game, [1, 3], "BLOCK", WHITE);
        insert(&mut game, [1, 5], "BLOCK", BLACK);
        let offered: Vec<bool> = (1..8u8)
            .map(|y| can_move(&mut game, [1, 0], [1, y]))
            .collect();
        assert_eq!(
            offered,
            vec![true, true, false, true, true, true, true],
            "not onto the ally, onto the enemy, and on past both"
        );

        // HUNTER stops once it reaches an enemy, and passes over allies.
        let mut game = game_with("HUNTER", [2, 0], WHITE);
        insert(&mut game, [2, 2], "BLOCK", WHITE);
        insert(&mut game, [2, 5], "BLOCK", BLACK);
        let offered: Vec<bool> = (1..8u8)
            .map(|y| can_move(&mut game, [2, 0], [2, y]))
            .collect();
        assert_eq!(
            offered,
            vec![true, false, true, true, true, false, false],
            "over the ally, onto the enemy, and no further"
        );
    }

    /// A 4×4 game where PAWN, in unlimited supply, is placed under `conditions`, the tests of
    /// the PAWN action after its PLACE choice.
    fn drop_spec(conditions: Value) -> String {
        let mut placing = vec![
            json!({ "choose": "PLACE", "in": { "region": "BOARD" } }),
            target_empty(),
        ];
        placing.extend(
            conditions
                .as_array()
                .cloned()
                .expect("conditions are a list"),
        );
        placing.push(json!({ "effect": "PLACE", "at": { "place": "TARGET" }, "code": "PAWN" }));
        json!({
            "format_version": 1,
            "name": "DROPS",
            "board": { "dimensions": [4, 4] },
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "pieces": [{ "code": "PAWN", "name": "pawn", "moves": [] }],
            "actions": [{ "name": "PAWN", "do": placing }]
        })
        .to_string()
    }

    #[test]
    fn the_tests_of_a_drop_action_must_all_hold() {
        let no_ally_pawn_on_the_file = json!({
            "test": "NOT",
            "of": {
                "test": "COUNT",
                "in": { "region": "LINE", "through": { "place": "TARGET" }, "along": [0, 1] },
                "where": { "owner": "ALLY", "codes": ["PAWN"] }
            }
        });
        let no_enemy_above = json!({
            "test": "NOT",
            "of": {
                "test": "COUNT",
                "in": { "region": "AT", "at": { "place": "TARGET", "offset": [0, 1] } },
                "where": { "owner": "ENEMY" }
            }
        });
        let spec = drop_spec(json!([no_ally_pawn_on_the_file, no_enemy_above]));
        let mut game = Game::from_spec(read(spec).expect("the drop spec is read"));
        insert(&mut game, [1, 0], "PAWN", WHITE);
        insert(&mut game, [2, 2], "PAWN", BLACK);

        let squares = drop_squares(&game, "PAWN");
        let offered = |at: [u8; 2]| squares.contains(&Position::from_coordinates(&at));
        for at in [[1, 1], [1, 2], [1, 3]] {
            assert!(!offered(at), "{at:?} is on a file with an ally pawn");
        }
        assert!(!offered([2, 1]), "[2,1] is under an enemy");
        for at in [[2, 0], [2, 3], [0, 0], [3, 3]] {
            assert!(offered(at), "{at:?} passes both conditions");
        }
    }

    #[test]
    fn zones_are_read_from_zones() {
        let spec = json!({
            "format_version": 1,
            "name": "ZONES",
            "board": { "dimensions": [4, 4] },
            "zones": [
                { "name": "CAMP", "places": { "WHITE": [[0, 3], "1,3"], "BLACK": [[0, 0]] } }
            ],
            "players": [
                {
                    "name": "WHITE",
                    "direction": [[1, 0], [0, 1]],
                    "starting_positions": [{ "piece": "RUNNER", "positions": [[0, 2], [1, 2], [2, 2]] }]
                },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "end_checks": [{
                "when": { "test": "SOME",
                          "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": ["RUNNER"] } },
                          "that": { "test": "IN_ZONE", "at": { "place": "EACH" }, "zone": "CAMP" } },
                "then": { "outcome": "WIN" }
            }],
            "pieces": [{
                "code": "RUNNER",
                "name": "runner",
                "moves": [{
                    "id": 0,
                    "step": [0, 1],
                    "do": steps(json!([
                        { "test": "IN_ZONE", "at": { "place": "TARGET" }, "zone": "CAMP" }
                    ]))
                }]
            }],
            "actions": [move_action()]
        })
        .to_string();
        let mut game = Game::from_spec(read(spec).expect("the zones spec is read"));

        assert!(!can_move(&mut game, [2, 2], [2, 3]), "[2,3] is not in CAMP");
        assert!(
            can_move(&mut game, [1, 2], [1, 3]),
            "[1,3] is in CAMP by name"
        );
        assert!(
            can_move(&mut game, [0, 2], [0, 3]),
            "[0,3] is in CAMP by coordinates"
        );
        go(&mut game, Position::from_coordinates(&[0, 3])).expect("the move is played");
        assert_eq!(
            game.state.phase,
            GamePhase::GameOver {
                winner: Some(WHITE.to_string())
            },
            "the runner reached CAMP"
        );
    }

    // ---------------------------------------------------------------------
    // Refusals when the spec is read
    // ---------------------------------------------------------------------

    /// A game on a 4×4 grid, with a zone PALACE, whose PROBE steps [0, 1] under `conditions`.
    fn grid_spec(conditions: Value) -> String {
        json!({
            "format_version": 1,
            "name": "BLOCKS",
            "board": { "dimensions": [4, 4] },
            "zones": [{ "name": "PALACE", "places": { "WHITE": [[0, 0]], "BLACK": [[3, 3]] } }],
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "pieces": [
                {
                    "code": "PROBE",
                    "name": "probe",
                    "moves": [{ "id": 0, "step": [0, 1], "do": steps(conditions) }]
                }
            ],
            "actions": [move_action()]
        })
        .to_string()
    }

    /// A game on a map of three points in a row, whose PROBE steps along `next` under
    /// `conditions`.
    fn map_spec(conditions: Value) -> String {
        json!({
            "format_version": 1,
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
            "players": [
                { "name": "A", "starting_positions": [] },
                { "name": "B", "starting_positions": [] }
            ],
            "turns": { "order": ["A", "B"] },
            "pieces": [
                {
                    "code": "PROBE",
                    "name": "probe",
                    "moves": [{ "id": 0, "step": "next", "do": steps(conditions) }]
                }
            ],
            "actions": [move_action()]
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
            ("[0, 2]", line_along(json!([0, 2]))),
            ("[2, 1]", line_along(json!([2, 1]))),
            ("[0, 0]", line_along(json!([0, 0]))),
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
            assert!(
                error.to_string().contains("move 0 of piece PROBE"),
                "the refusal of {named} says where: {error}"
            );
        }
    }

    /// `COUNT` of any piece on the line through `TARGET` along `along`.
    fn line_along(along: Value) -> Value {
        json!({
            "test": "COUNT",
            "in": { "region": "LINE", "through": { "place": "TARGET" }, "along": along }
        })
    }

    #[test]
    fn refusals_reach_promotions_drops_and_repeats() {
        let bad = count_at(json!({ "place": "HERE" }));
        let mut promotion: Value =
            serde_json::from_str(&grid_spec(json!([]))).expect("the spec is JSON");
        promotion["pieces"][0]["moves"][0]["do"] = json!([
            target_empty(),
            step_to_target(),
            { "if": bad, "then": [
                { "effect": "SET_CODE", "at": { "place": "TARGET" }, "code": "PROBE" }
            ] }
        ]);
        let mut until: Value = serde_json::from_str(&grid_spec(json!([]))).expect("JSON");
        until["pieces"][0]["moves"][0]["repeat"] = json!({ "until": bad, "loop": true });
        let mut drop = serde_json::from_str::<Value>(&drop_spec(json!([]))).expect("JSON");
        drop["actions"][0]["do"] = json!([
            { "choose": "PLACE", "in": { "region": "BOARD" } },
            bad,
            { "effect": "PLACE", "at": { "place": "TARGET" }, "code": "PAWN" }
        ]);

        for (where_, spec, refused_as_a_step) in [
            ("step 3 of move 0 of piece PROBE", promotion, true),
            ("the repeat of move 0 of piece PROBE", until, false),
            ("step 2 of action PAWN", drop, true),
        ] {
            let result = read(spec.to_string());
            let error = result.err().unwrap_or_else(|| panic!("{where_}: refused"));
            assert!(
                if refused_as_a_step {
                    matches!(error, GameSpecError::InvalidStep(_))
                } else {
                    matches!(error, GameSpecError::InvalidTest(_))
                },
                "{where_}: refused as {}: {error}",
                if refused_as_a_step {
                    "a step"
                } else {
                    "a test"
                }
            );
            assert!(
                error.to_string().contains(where_),
                "the refusal says {where_}: {error}"
            );
        }
    }

    #[test]
    fn a_piece_in_zone_win_needs_a_declared_zone() {
        let in_zone = |zone: Value| {
            json!({
                "when": { "test": "SOME",
                          "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": ["PROBE"] } },
                          "that": zone },
                "then": { "outcome": "WIN" }
            })
        };
        let with_win = |win: Value| {
            let mut spec: Value = serde_json::from_str(&grid_spec(json!([]))).expect("JSON");
            spec["end_checks"] = json!([win]);
            read(spec.to_string())
        };
        assert!(
            with_win(in_zone(
                json!({ "test": "IN_ZONE", "at": { "place": "EACH" }, "zone": "PALACE" })
            ))
            .is_ok(),
            "PALACE is declared"
        );
        for (named, win) in [
            (
                "KREMLIN",
                in_zone(json!({ "test": "IN_ZONE", "at": { "place": "EACH" }, "zone": "KREMLIN" })),
            ),
            (
                "zone",
                in_zone(json!({ "test": "IN_ZONE", "at": { "place": "EACH" } })),
            ),
        ] {
            let result = with_win(win);
            assert!(
                matches!(&result, Err(error) if error.to_string().contains(named)),
                "{named}: {:?}",
                result.err()
            );
        }
    }

    #[test]
    fn start_in_the_tests_of_a_drop_action_is_refused() {
        let at_start = count_at(json!({ "place": "START" }));
        let nested = json!({
            "test": "NOT",
            "of": {
                "test": "COUNT",
                "in": { "region": "AT", "at": { "place": "START", "offset": [0, 1] } }
            }
        });
        for test in [at_start, nested] {
            let result = read(drop_spec(json!([test])));
            let error = result.err().expect("START in a drop action is refused");
            assert!(
                matches!(error, GameSpecError::InvalidStep(_)),
                "refused as a step: {error}"
            );
            assert!(
                error.to_string().contains("START") && error.to_string().contains("action PAWN"),
                "the refusal names START and where: {error}"
            );
        }
        assert!(
            read(drop_spec(json!([count_at(json!({ "place": "TARGET" }))]))).is_ok(),
            "TARGET is the drop square"
        );
    }

    /// `grid_spec` whose PROBE steps `step`, sliding when `slides`, where no piece may stand on
    /// the PATH from `from` to `to`.
    fn path_spec(step: Value, slides: bool, from: Value, to: Value) -> String {
        let mut spec: Value = serde_json::from_str(&grid_spec(json!([{
            "test": "COUNT",
            "in": { "region": "PATH", "from": from, "to": to },
            "min": 0,
            "max": 0
        }])))
        .expect("JSON");
        let movement = &mut spec["pieces"][0]["moves"][0];
        movement["step"] = step;
        if slides {
            movement["repeat"] = json!({ "loop": true });
        }
        spec.to_string()
    }

    #[test]
    fn a_path_whose_line_is_not_fixed_when_read_is_refused() {
        let start = json!({ "place": "START" });
        let target = json!({ "place": "TARGET" });
        let start_up = json!({ "place": "START", "offset": [0, 1] });
        let cases = vec![
            (
                "same place",
                json!([0, 1]),
                false,
                start.clone(),
                start.clone(),
            ),
            (
                "same place",
                json!([0, 1]),
                false,
                start_up.clone(),
                start_up.clone(),
            ),
            (
                "same place",
                json!([0, 1]),
                false,
                start_up.clone(),
                target.clone(),
            ),
            (
                "offset",
                json!([0, 1]),
                true,
                start_up.clone(),
                target.clone(),
            ),
            (
                "offset",
                json!([0, 1]),
                true,
                start.clone(),
                json!({ "place": "TARGET", "offset": [0, -1] }),
            ),
            (
                "straight line",
                json!([0, 1]),
                false,
                start.clone(),
                json!({ "place": "START", "offset": [2, 1] }),
            ),
            (
                "straight line",
                json!([0, 1]),
                false,
                target.clone(),
                json!({ "place": "TARGET", "offset": [1, 2] }),
            ),
            (
                "straight line",
                json!([2, 1]),
                false,
                start.clone(),
                target.clone(),
            ),
            (
                "straight line",
                json!([2, 1]),
                true,
                start.clone(),
                target.clone(),
            ),
        ];
        for (named, step, slides, from, to) in cases {
            let shape = format!("{from} to {to} on a step of {step}, sliding: {slides}");
            let result = read(path_spec(step, slides, from, to));
            let error = result.err().unwrap_or_else(|| panic!("{shape} is refused"));
            assert!(
                matches!(error, GameSpecError::InvalidTest(_)),
                "{shape}: refused as a test: {error}"
            );
            assert!(
                error.to_string().contains(named)
                    && error.to_string().contains("move 0 of piece PROBE"),
                "{shape}: the refusal says {named} and where: {error}"
            );
        }
    }

    #[test]
    fn a_path_whose_line_is_fixed_when_read_is_read() {
        let start = json!({ "place": "START" });
        let target = json!({ "place": "TARGET" });
        let cases = vec![
            // Castling: nothing between the king and its rook.
            (
                json!([2, 0]),
                false,
                start.clone(),
                json!({ "place": "START", "offset": [3, 0] }),
            ),
            // The cannon: one piece between it and the square it slides to.
            (json!([0, 1]), true, start.clone(), target.clone()),
            (json!([0, 3]), false, target.clone(), start.clone()),
            (
                json!([1, 0]),
                false,
                start.clone(),
                json!({ "place": "TARGET", "offset": [1, 0] }),
            ),
        ];
        for (step, slides, from, to) in cases {
            let shape = format!("{from} to {to} on a step of {step}, sliding: {slides}");
            let result = read(path_spec(step, slides, from, to));
            assert!(result.is_ok(), "{shape} is read: {:?}", result.err());
        }
        let in_a_drop = json!([{
            "test": "COUNT",
            "in": {
                "region": "PATH",
                "from": { "place": "TARGET" },
                "to": { "place": "TARGET", "offset": [0, 2] }
            },
            "min": 0,
            "max": 0
        }]);
        let result = read(drop_spec(in_a_drop));
        let error = result.expect_err("an action has no step to fix a PATH's line");
        assert!(
            matches!(&error, GameSpecError::InvalidStep(why) if why.contains("a PATH")),
            "a PATH in a drop action is refused as a step: {error}"
        );
    }

    /// `grid_spec` with `zones` written after its `PALACE` zone.
    fn grid_spec_with_zones(zones: Value) -> String {
        let mut spec: Value = serde_json::from_str(&grid_spec(json!([]))).expect("JSON");
        let Value::Array(zones) = zones else {
            panic!("zones are a list");
        };
        spec["zones"]
            .as_array_mut()
            .expect("grid_spec has zones")
            .extend(zones);
        spec.to_string()
    }

    #[test]
    fn a_zone_named_twice_is_refused() {
        let camp = json!({ "name": "CAMP", "places": { "WHITE": [[0, 1]] } });
        let palace = json!({ "name": "PALACE", "places": { "WHITE": [[0, 1]] } });
        assert!(
            read(grid_spec_with_zones(json!([camp]))).is_ok(),
            "a zone beside a zone of another name"
        );
        for (named, zones) in [("CAMP", json!([camp, camp])), ("PALACE", json!([palace]))] {
            let result = read(grid_spec_with_zones(zones));
            assert!(
                matches!(&result, Err(GameSpecError::DuplicateZone(name)) if name == named),
                "{named} named twice: {:?}",
                result.err()
            );
        }
    }

    #[test]
    fn a_zone_for_a_player_the_game_lacks_is_refused() {
        let camp = json!({ "name": "CAMP", "places": { "WHITE": [[0, 1]], "RED": [[0, 2]] } });
        let result = read(grid_spec_with_zones(json!([camp])));
        let error = result.expect_err("RED is not a player of the game");
        assert!(
            matches!(&error, GameSpecError::UnknownPlayerInZone { zone, player } if zone == "CAMP" && player == "RED"),
            "the refusal names the zone and the player: {error}"
        );
        assert!(
            error.to_string().contains("CAMP") && error.to_string().contains("RED"),
            "{error}"
        );
    }

    #[test]
    fn zone_places_are_checked_like_positions() {
        let zone = |places: Value| json!([{ "name": "CAMP", "places": { "WHITE": places } }]);
        let result = read(grid_spec_with_zones(zone(json!(["9,9"]))));
        assert!(
            matches!(&result, Err(GameSpecError::UnknownPosition(name)) if name == "9,9"),
            "a grid zone names a square the grid lacks: {:?}",
            result.err()
        );

        let map_zone = |places: Value| json!([{ "name": "CAMP", "places": { "A": places } }]);
        let mut on_map: Value = serde_json::from_str(&map_spec(json!([]))).expect("JSON");
        on_map["zones"] = map_zone(json!(["a", "b"]));
        assert!(read(on_map.to_string()).is_ok(), "place names on a map");
        on_map["zones"] = map_zone(json!(["z"]));
        let result = read(on_map.to_string());
        assert!(
            matches!(&result, Err(GameSpecError::UnknownPosition(name)) if name == "z"),
            "a map zone names a place the map lacks: {:?}",
            result.err()
        );
        on_map["zones"] = map_zone(json!([[0, 1]]));
        let result = read(on_map.to_string());
        assert!(
            matches!(&result, Err(GameSpecError::NotOnAMap(_))),
            "coordinates on a map: {:?}",
            result.err()
        );

        let mut connected: Value =
            serde_json::from_str(&grid_spec_with_zones(zone(json!([[0, 1]])))).expect("JSON");
        connected["end_checks"] = json!([{
            "when": { "test": "SOME",
                      "in": { "region": "BOARD", "where": { "owner": "ALLY" } },
                      "that": { "test": "SOME",
                                "in": { "region": "GROUP", "of": { "place": "EACH" },
                                        "through": "orthogonal" },
                                "that": { "test": "IN_ZONE", "at": { "place": "EACH" },
                                          "zone": "CAMP" } } },
            "then": { "outcome": "WIN" }
        }]);
        assert!(
            read(connected.to_string()).is_ok(),
            "a connection may touch a zone written under zones"
        );
    }

    #[test]
    fn refusals_reach_into_nested_tests_and_actions() {
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
        spec["actions"][0]["do"]
            .as_array_mut()
            .expect("steps")
            .push(count_at(json!({ "place": "HERE" })));
        assert!(
            read(spec.to_string()).is_err(),
            "a place in an action's tests"
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
