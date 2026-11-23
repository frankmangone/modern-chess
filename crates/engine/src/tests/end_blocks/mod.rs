//! The tests that slice 4 adds: `SOME`, `PASSES`, `COUNTER`, `MOVED`, `REPEATS` with `times`,
//! the filter `except` and `ATTACKED keeping`, with their refusals when the spec is read.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::rules::test_block::{judge, AfterTurn, Test, TestContext};
    use crate::logic::{Game, Labels, Piece};
    use crate::shared::Position;
    use crate::specs::game::test_block::TestSpec;
    use crate::specs::{parse_game_spec, parse_game_spec_str, GameSpec, GameSpecError, PlayerSpec};
    use crate::tests::support::{pass, pass_action};

    const WHITE: &str = "WHITE";
    const BLACK: &str = "BLACK";

    fn read(spec: &Value) -> Result<GameSpec, GameSpecError> {
        parse_game_spec_str(&spec.to_string())
    }

    fn at(place: &str) -> Value {
        json!({ "place": place })
    }

    /// A 4×4 game with a PROBE that steps [0, 1] onto a square no ally holds, and a PAWN and a
    /// KING that do not move. The counter `X` is declared.
    fn small_spec(actions: Value) -> Value {
        json!({
            "format_version": 1,
            "name": "END_BLOCKS",
            "board": { "dimensions": [4, 4] },
            "counters": ["X"],
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "pieces": [
                { "code": "PROBE", "name": "probe", "moves": [{
                    "id": 0, "step": [0, 1],
                    "do": [
                        { "test": "NOT", "of": { "test": "COUNT",
                            "in": { "region": "AT", "at": at("TARGET") },
                            "where": { "owner": "ALLY" } } },
                        { "effect": "MOVE", "from": at("START"), "to": at("TARGET") }
                    ]
                }] },
                { "code": "PAWN", "name": "pawn", "moves": [] },
                { "code": "KING", "name": "king", "moves": [] }
            ],
            "actions": actions
        })
    }

    /// The action `A`: choose an ally piece, one of its moves, then `closing`.
    fn move_then(closing: Value) -> Value {
        small_spec(json!([{ "name": "A", "do": [
            { "choose": "PIECE", "where": { "owner": "ALLY" } },
            { "choose": "MOVE" },
            closing
        ] }]))
    }

    struct Fixture {
        game: Game,
        players: Vec<PlayerSpec>,
    }

    fn fixture(spec: &Value) -> Fixture {
        let spec = read(spec).unwrap_or_else(|error| panic!("the spec is read: {error}"));
        Fixture {
            players: spec.players.clone(),
            game: Game::from_spec(spec),
        }
    }

    fn put(game: &mut Game, at: Position, code: &str, player: &str) {
        game.state
            .pieces
            .insert(at, Piece::new(code.to_string(), player.to_string()));
    }

    /// Whether `test` holds for `player` on the pieces as they stand.
    fn holds(fixture: &Fixture, test: Value, player: &str, after_turn: Option<AfterTurn>) -> bool {
        let spec: TestSpec = serde_json::from_value(test).expect("the test is read");
        let test = Test::build(&spec, &fixture.players, None, &mut Labels::default());
        let nowhere = Position::default();
        judge(
            &test,
            &TestContext {
                game: &fixture.game,
                pieces: crate::logic::PiecesView::of(&fixture.game.state.pieces),
                player,
                start: None,
                target: &nowhere,
                each: None,
                removed: &[],
                moved: &[],
                after_turn,
                last: None,
            },
        )
    }

    fn enemy_next_to_an_ally() -> Value {
        json!({ "test": "SOME",
            "in": { "region": "BOARD", "where": { "owner": "ALLY" } },
            "that": { "test": "COUNT",
                "in": { "region": "AROUND", "of": at("EACH"), "through": "orthogonal" },
                "where": { "owner": "ENEMY" } } })
    }

    #[test]
    fn some_holds_when_a_place_of_the_region_passes() {
        let mut f = fixture(&small_spec(json!([pass_action()])));
        put(&mut f.game, pos![1, 1], "PAWN", WHITE);
        put(&mut f.game, pos![1, 2], "PAWN", BLACK);
        assert!(holds(&f, enemy_next_to_an_ally(), WHITE, None));
        f.game.state.pieces.remove(&pos![1, 2]);
        put(&mut f.game, pos![3, 3], "PAWN", BLACK);
        assert!(!holds(&f, enemy_next_to_an_ally(), WHITE, None));
    }

    #[test]
    fn some_over_no_place_does_not_hold() {
        let f = fixture(&small_spec(json!([pass_action()])));
        assert!(!holds(&f, enemy_next_to_an_ally(), WHITE, None));
    }

    #[test]
    fn a_some_inside_a_some_sees_the_inner_each_and_takes_its_region_from_the_outer() {
        let nested = json!({ "test": "SOME",
            "in": { "region": "BOARD", "where": { "owner": "ALLY" } },
            "that": { "test": "SOME",
                "in": { "region": "AROUND", "of": at("EACH"), "through": "orthogonal" },
                "that": { "test": "COUNT",
                    "in": { "region": "AT", "at": at("EACH") },
                    "where": { "owner": "ENEMY" } } } });
        let mut f = fixture(&small_spec(json!([pass_action()])));
        put(&mut f.game, pos![1, 1], "PAWN", WHITE);
        put(&mut f.game, pos![1, 3], "PAWN", BLACK);
        assert!(!holds(&f, nested.clone(), WHITE, None), "two squares away");
        put(&mut f.game, pos![1, 2], "PAWN", BLACK);
        assert!(holds(&f, nested, WHITE, None), "next to the ally");
    }

    #[test]
    fn passes_counts_the_trailing_turns_that_changed_nothing() {
        let mut f = fixture(&small_spec(json!([pass_action()])));
        let two = json!({ "test": "PASSES", "in_a_row": 2 });
        assert!(!holds(&f, two.clone(), WHITE, None));
        pass(&mut f.game).expect("a pass");
        assert!(!holds(&f, two.clone(), WHITE, None), "one pass");
        pass(&mut f.game).expect("a pass");
        assert!(holds(&f, two, WHITE, None), "two passes in a row");
    }

    #[test]
    fn counter_reads_the_judging_players_counter() {
        let mut f = fixture(&small_spec(json!([pass_action()])));
        f.game
            .state
            .counters
            .insert(WHITE.to_string(), [("X".to_string(), 2)].into());
        let test = json!({ "test": "COUNTER", "name": "X", "min": 2, "max": 3 });
        assert!(holds(&f, test.clone(), WHITE, None));
        assert!(!holds(&f, test, BLACK, None), "BLACK's counter is 0");
        let at_most_one = json!({ "test": "COUNTER", "name": "X", "max": 1 });
        assert!(holds(&f, at_most_one, BLACK, None), "min defaults to 0");
    }

    #[test]
    fn moved_sees_the_piece_that_travelled_and_not_the_one_it_took() {
        let ways_with = |code: &str| {
            let spec = move_then(json!({ "test": "MOVED", "where": { "codes": [code] } }));
            let mut f = fixture(&spec);
            put(&mut f.game, pos![0, 0], "PROBE", WHITE);
            put(&mut f.game, pos![0, 1], "PAWN", BLACK);
            f.game.compute_ways().len()
        };
        assert_eq!(
            ways_with("PROBE"),
            1,
            "the probe moved while taking the pawn"
        );
        assert_eq!(ways_with("PAWN"), 0, "a piece that is taken has not moved");
    }

    #[test]
    fn repeats_with_times_counts_the_recorded_positions_after_a_turn() {
        let mut f = fixture(&small_spec(json!([pass_action()])));
        put(&mut f.game, pos![1, 1], "PAWN", WHITE);
        let key = f.game.position_key();
        f.game.state.position_hashes = vec![key.clone(), key.clone()];
        let settled = Some(AfterTurn { stuck: false });
        let times = |n: u16| json!({ "test": "REPEATS", "times": n });
        assert!(holds(&f, times(2), WHITE, settled));
        assert!(!holds(&f, times(3), WHITE, settled));
        f.game.state.position_hashes.push(key);
        assert!(holds(&f, times(3), WHITE, settled), "recorded three times");
    }

    #[test]
    fn except_takes_every_code_but_the_ones_written() {
        let mut f = fixture(&small_spec(json!([pass_action()])));
        put(&mut f.game, pos![0, 0], "KING", WHITE);
        let only_a_king = json!({ "test": "COUNT",
            "in": { "region": "BOARD" }, "where": { "owner": "ALLY", "except": ["KING"] } });
        assert!(!holds(&f, only_a_king.clone(), WHITE, None));
        put(&mut f.game, pos![2, 2], "PAWN", WHITE);
        assert!(holds(&f, only_a_king, WHITE, None));
    }

    /// Atomic without its check rule, so that only a test's own `keeping` decides what counts.
    fn atomic_without_leader() -> Fixture {
        let mut spec = parse_game_spec("./specs/atomic.json").expect("atomic loads");
        spec.every_action.clear();
        let mut f = Fixture {
            players: spec.players.clone(),
            game: Game::from_spec(spec),
        };
        f.game.state.pieces.clear();
        // BLACK's rook looks along the rank at WHITE's king; BLACK's king stands right beside.
        put(&mut f.game, pos![4, 3], "KING", WHITE);
        put(&mut f.game, pos![4, 4], "KING", BLACK);
        put(&mut f.game, pos![0, 3], "ROOK", BLACK);
        f
    }

    fn white_king_attacked(keeping: Option<Value>) -> Value {
        let mut test = json!({ "test": "ATTACKED", "by": "ENEMY",
            "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": ["KING"] } } });
        if let Some(keeping) = keeping {
            test["keeping"] = keeping;
        }
        test
    }

    #[test]
    fn attacked_keeping_ignores_a_capture_that_loses_the_piece_kept() {
        let f = atomic_without_leader();
        assert!(
            holds(&f, white_king_attacked(None), WHITE, None),
            "without `keeping` every capture counts"
        );
        assert!(
            !holds(
                &f,
                white_king_attacked(Some(json!({ "codes": ["KING"] }))),
                WHITE,
                None
            ),
            "the rook's capture would blow up BLACK's own king"
        );
    }

    fn refused(spec: Value) -> String {
        match read(&spec) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("the spec was not refused"),
        }
    }

    #[test]
    fn a_counter_that_is_not_declared_is_refused() {
        let message = refused(move_then(json!({ "test": "COUNTER", "name": "Y" })));
        assert!(
            message.contains("`Y`") && message.contains("counters"),
            "{message}"
        );
    }

    #[test]
    fn counter_names_are_not_empty_and_not_repeated() {
        for counters in [json!(["X", "X"]), json!([""])] {
            let mut spec = small_spec(json!([pass_action()]));
            spec["counters"] = counters;
            let message = refused(spec);
            assert!(message.contains("counters"), "{message}");
        }
    }

    #[test]
    fn a_counter_test_with_its_min_above_its_max_is_refused() {
        let test = json!({ "test": "COUNTER", "name": "X", "min": 3, "max": 1 });
        assert!(refused(move_then(test)).contains("min above its max"));
    }

    #[test]
    fn passes_and_repeats_with_a_zero_are_refused() {
        let message = refused(move_then(json!({ "test": "PASSES", "in_a_row": 0 })));
        assert!(message.contains("in_a_row"), "{message}");
        let message = refused(move_then(json!({ "test": "REPEATS", "times": 0 })));
        assert!(message.contains("times"), "{message}");
    }

    #[test]
    fn keeping_with_an_owner_or_empty_is_refused() {
        for keeping in [json!({ "owner": "ALLY" }), json!({ "empty": true })] {
            let mut test = white_king_attacked(None);
            test["keeping"] = keeping;
            let message = refused(move_then(test));
            assert!(message.contains("keeping"), "{message}");
        }
    }

    #[test]
    fn except_with_empty_or_an_unknown_code_is_refused() {
        let with = |filter: Value| {
            move_then(json!({ "test": "COUNT", "in": { "region": "BOARD", "where": filter } }))
        };
        let message = refused(with(json!({ "empty": true, "except": ["PAWN"] })));
        assert!(message.contains("except"), "{message}");
        let message = refused(with(json!({ "except": ["DRAGON"] })));
        assert!(message.contains("DRAGON"), "{message}");
    }

    #[test]
    fn a_moved_with_an_unknown_code_is_refused() {
        let message = refused(move_then(
            json!({ "test": "MOVED", "where": { "codes": ["DRAGON"] } }),
        ));
        assert!(message.contains("DRAGON"), "{message}");
    }

    #[test]
    fn a_some_names_the_each_only_inside_its_that() {
        let outside = json!({ "test": "COUNT", "in": { "region": "AT", "at": at("EACH") } });
        assert!(refused(move_then(outside)).contains("EACH"));
    }
}
