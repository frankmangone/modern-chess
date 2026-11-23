//! Placing stones from a supply, capturing groups with no liberties, passing and area scoring,
//! exercised through a 9x9 Go spec. None of the rules below knows about Go: each is written in
//! the spec (an action that places a stone, an `every_action` entry for liberties and repeated
//! positions, an action with no steps to pass and the `HIGHEST` end check over a `score`).

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Answer, Game, GameError, GamePhase, GameTransition, Piece};
    use crate::shared::Position;
    use crate::specs::{parse_game_spec, parse_game_spec_str};
    use crate::tests::support::{
        self, choosing, chosen, drop_squares, move_action, moves_from, onto_an_empty_square,
        pass_action, phase_is_choosing, pick, pick_hand, promotion_options,
    };

    const STONE: &str = "STONE";

    fn load_go() -> Game {
        parse_game_spec("./specs/go.json")
            .map(Game::from_spec)
            .expect("Failed to load go spec")
    }

    fn put(game: &mut Game, x: u8, y: u8, player: &str) {
        game.state.pieces.insert(
            pos![x, y],
            Piece::new(STONE.to_string(), player.to_string()),
        );
    }

    fn owner(game: &Game, x: u8, y: u8) -> Option<&str> {
        game.state
            .pieces
            .get(&pos![x, y])
            .map(|p| p.player.as_str())
    }

    fn place(game: &mut Game, x: u8, y: u8) -> Result<(), GameError> {
        support::drop(game, STONE, pos![x, y])
    }

    fn can_place(game: &mut Game, x: u8, y: u8) -> bool {
        drop_squares(game, STONE).contains(&pos![x, y])
    }

    fn pass(game: &mut Game) -> Result<(), GameError> {
        support::pass(game)
    }

    fn lost(game: &Game, player: &str) -> u32 {
        game.state
            .captured
            .get(player)
            .and_then(|lost| lost.get(STONE))
            .copied()
            .unwrap_or(0)
    }

    // -----------------------------------------------------------------------
    // Placing from an unlimited supply
    // -----------------------------------------------------------------------

    #[test]
    fn test_go_spec_loads() {
        load_go();
    }

    #[test]
    fn test_a_piece_with_a_supply_can_be_placed_with_nothing_in_hand() {
        let mut game = load_go();
        assert!(game.hand().is_empty());

        place(&mut game, 4, 4).unwrap();

        assert_eq!(owner(&game, 4, 4), Some("BLACK"));
        assert_eq!(game.current_player(), "WHITE");
    }

    #[test]
    fn test_every_empty_point_is_offered_and_occupied_ones_are_not() {
        let mut game = load_go();
        put(&mut game, 0, 0, "WHITE");

        let offered = drop_squares(&game, &STONE);

        assert_eq!(offered.len(), 80);
        assert!(!offered.contains(&pos![0, 0]));
    }

    #[test]
    fn test_the_supply_never_runs_out() {
        let mut game = load_go();
        for turn in 0..30u8 {
            place(&mut game, turn % 9, turn / 9).unwrap();
        }
        assert_eq!(game.history().len(), 30);
        assert!(game.hand().is_empty());
    }

    #[test]
    fn test_a_piece_without_a_supply_still_needs_the_hand() {
        let mut game = parse_game_spec("./specs/chess.json")
            .map(Game::from_spec)
            .unwrap();
        let result = pick_hand(&mut game, "PAWN");
        assert!(matches!(result, Err(GameError::NotAnOption)));
    }

    // -----------------------------------------------------------------------
    // Capturing groups without liberties
    // -----------------------------------------------------------------------

    #[test]
    fn test_a_stone_with_no_liberties_is_captured() {
        let mut game = load_go();
        put(&mut game, 4, 4, "WHITE");
        put(&mut game, 3, 4, "BLACK");
        put(&mut game, 5, 4, "BLACK");
        put(&mut game, 4, 3, "BLACK");

        place(&mut game, 4, 5).unwrap();

        assert_eq!(owner(&game, 4, 4), None, "the white stone is taken");
        assert_eq!(lost(&game, "WHITE"), 1);
        assert_eq!(owner(&game, 4, 5), Some("BLACK"));
    }

    #[test]
    fn test_a_stone_on_the_edge_needs_fewer_stones_around_it() {
        let mut game = load_go();
        put(&mut game, 0, 0, "WHITE");
        put(&mut game, 1, 0, "BLACK");

        place(&mut game, 0, 1).unwrap();

        assert_eq!(owner(&game, 0, 0), None);
    }

    #[test]
    fn test_a_connected_group_is_captured_together() {
        let mut game = load_go();
        put(&mut game, 0, 0, "WHITE");
        put(&mut game, 1, 0, "WHITE");
        put(&mut game, 0, 1, "WHITE");
        put(&mut game, 2, 0, "BLACK");
        put(&mut game, 1, 1, "BLACK");
        // The last liberty of the three is (0,2).
        place(&mut game, 0, 2).unwrap();

        assert_eq!(owner(&game, 0, 0), None);
        assert_eq!(owner(&game, 1, 0), None);
        assert_eq!(owner(&game, 0, 1), None);
        assert_eq!(lost(&game, "WHITE"), 3);
    }

    #[test]
    fn test_a_group_with_a_liberty_left_is_not_captured() {
        let mut game = load_go();
        put(&mut game, 4, 4, "WHITE");
        put(&mut game, 4, 5, "WHITE");
        put(&mut game, 3, 4, "BLACK");
        put(&mut game, 5, 4, "BLACK");
        put(&mut game, 4, 3, "BLACK");

        place(&mut game, 3, 5).unwrap();

        assert_eq!(owner(&game, 4, 4), Some("WHITE"));
        assert_eq!(lost(&game, "WHITE"), 0);
    }

    #[test]
    fn test_stones_only_touching_on_a_diagonal_are_separate_groups() {
        let mut game = load_go();
        put(&mut game, 0, 0, "WHITE");
        put(&mut game, 1, 1, "WHITE");
        put(&mut game, 1, 0, "BLACK");

        place(&mut game, 0, 1).unwrap();

        assert_eq!(
            owner(&game, 0, 0),
            None,
            "the corner stone has no liberties"
        );
        assert_eq!(
            owner(&game, 1, 1),
            Some("WHITE"),
            "the diagonal one is its own group"
        );
    }

    #[test]
    fn test_the_mover_s_own_stones_are_never_removed_by_a_capture() {
        let mut game = load_go();
        put(&mut game, 0, 0, "WHITE");
        put(&mut game, 1, 0, "BLACK");

        place(&mut game, 0, 1).unwrap();

        assert_eq!(owner(&game, 1, 0), Some("BLACK"));
        assert_eq!(owner(&game, 0, 1), Some("BLACK"));
    }

    // -----------------------------------------------------------------------
    // Suicide
    // -----------------------------------------------------------------------

    fn walled_in_corner(game: &mut Game) {
        put(game, 1, 0, "WHITE");
        put(game, 0, 1, "WHITE");
    }

    #[test]
    fn test_a_placement_that_leaves_no_liberties_is_refused() {
        let mut game = load_go();
        walled_in_corner(&mut game);

        assert!(!can_place(&mut game, 0, 0));
        assert!(can_place(&mut game, 5, 5));
    }

    #[test]
    fn test_a_refused_suicide_changes_nothing() {
        let mut game = load_go();
        walled_in_corner(&mut game);

        let result = place(&mut game, 0, 0);

        assert!(matches!(result, Err(GameError::NotAnOption)));
        assert_eq!(owner(&game, 0, 0), None);
        assert_eq!(game.current_player(), "BLACK");
    }

    #[test]
    fn test_filling_the_last_liberty_is_allowed_when_it_captures() {
        // ```text
        // y=2   W  B  .
        // y=1   B  W  B
        // y=0   .  W  B
        //       x=0 1  2
        // ```
        // A black stone at (0,0) would have no liberty of its own, but it takes the white pair
        // at (1,0) and (1,1), whose only liberty it was.
        let mut game = load_go();
        for (x, y) in [(1, 0), (1, 1), (0, 2)] {
            put(&mut game, x, y, "WHITE");
        }
        for (x, y) in [(0, 1), (1, 2), (2, 1), (2, 0)] {
            put(&mut game, x, y, "BLACK");
        }

        assert!(can_place(&mut game, 0, 0));
        place(&mut game, 0, 0).unwrap();

        assert_eq!(owner(&game, 1, 0), None);
        assert_eq!(owner(&game, 1, 1), None);
        assert_eq!(owner(&game, 0, 0), Some("BLACK"));
        assert_eq!(lost(&game, "WHITE"), 2);
    }

    /// `go.json` read as JSON, for tests that change its rule.
    fn go_spec() -> Value {
        let text = std::fs::read_to_string("./specs/go.json").unwrap();
        serde_json::from_str(&text).unwrap()
    }

    /// The steps of the rule on stones in `go_spec`: captures, the liberty test, `NOT REPEATS`.
    fn rule_steps(spec: &mut Value) -> &mut Vec<Value> {
        spec["every_action"][0]["do"].as_array_mut().unwrap()
    }

    #[test]
    fn suicide_is_allowed_when_the_spec_removes_the_group_instead_of_refusing() {
        let mut spec = go_spec();
        // Instead of refusing a group of TARGET with no liberty, remove it.
        rule_steps(&mut spec)[1] = json!({
            "if": { "test": "COUNT", "min": 0, "max": 0, "where": { "empty": true },
                    "in": { "region": "AROUND", "through": "orthogonal", "of": {
                        "region": "GROUP", "of": { "place": "TARGET" }, "through": "orthogonal",
                        "where": { "codes": ["STONE"] } } } },
            "then": [{ "effect": "FOR_EACH", "in": {
                "region": "GROUP", "of": { "place": "TARGET" }, "through": "orthogonal",
                "where": { "codes": ["STONE"] } },
                "do": [{ "effect": "REMOVE", "at": { "place": "EACH" } }] }]
        });
        let spec = spec.to_string();
        let mut game = Game::from_spec(parse_game_spec_str(&spec).unwrap());
        // BLACK already has a stone at (0,1); the corner joins it into a group that is shut in.
        for (x, y) in [(1, 0), (1, 1), (0, 2)] {
            put(&mut game, x, y, "WHITE");
        }
        put(&mut game, 0, 1, "BLACK");

        assert!(can_place(&mut game, 0, 0));
        place(&mut game, 0, 0).unwrap();

        assert_eq!(
            owner(&game, 0, 0),
            None,
            "the stone took itself off the board"
        );
        assert_eq!(owner(&game, 0, 1), None, "and the group it joined");
        assert_eq!(
            lost(&game, "BLACK"),
            1,
            "the stone that was already there is lost"
        );
        assert_eq!(game.current_player(), "WHITE");
    }

    // -----------------------------------------------------------------------
    // Repeating a position
    // -----------------------------------------------------------------------

    /// The classic ko. WHITE has just placed the stone at (1,1), and BLACK is about to capture
    /// it by playing (2,1).
    ///
    /// ```text
    /// y=2   .  B  W  .
    /// y=1   B  W  .  W
    /// y=0   .  B  W  .
    ///       x=0 1  2  3
    /// ```
    fn ko() -> Game {
        let mut game = load_go();
        for (x, y) in [(0, 1), (1, 0), (1, 2)] {
            put(&mut game, x, y, "BLACK");
        }
        for (x, y) in [(2, 0), (2, 2), (3, 1)] {
            put(&mut game, x, y, "WHITE");
        }
        game.state.current_turn = 1; // WHITE to move
        place(&mut game, 1, 1).unwrap();
        game
    }

    #[test]
    fn test_the_stone_that_just_captured_cannot_be_taken_straight_back() {
        let mut game = ko();
        place(&mut game, 2, 1).unwrap(); // BLACK takes the white stone
        assert_eq!(owner(&game, 1, 1), None);

        assert!(
            !can_place(&mut game, 1, 1),
            "retaking would bring back the position from two turns ago"
        );
    }

    #[test]
    fn test_the_retake_is_allowed_once_the_board_has_changed_elsewhere() {
        let mut game = ko();
        place(&mut game, 2, 1).unwrap();

        place(&mut game, 7, 7).unwrap(); // WHITE plays a threat elsewhere
        place(&mut game, 7, 6).unwrap(); // BLACK answers it

        assert!(can_place(&mut game, 1, 1));
    }

    #[test]
    fn test_repetition_is_only_checked_when_the_spec_asks_for_it() {
        let mut spec = go_spec();
        rule_steps(&mut spec).pop(); // the closing NOT REPEATS
        let spec = spec.to_string();
        let mut game = Game::from_spec(parse_game_spec_str(&spec).unwrap());
        for (x, y) in [(0, 1), (1, 0), (1, 2)] {
            put(&mut game, x, y, "BLACK");
        }
        for (x, y) in [(2, 0), (2, 2), (3, 1)] {
            put(&mut game, x, y, "WHITE");
        }
        game.state.current_turn = 1;
        place(&mut game, 1, 1).unwrap();
        place(&mut game, 2, 1).unwrap();

        assert!(can_place(&mut game, 1, 1));
    }

    // -----------------------------------------------------------------------
    // Passing
    // -----------------------------------------------------------------------

    #[test]
    fn test_a_pass_hands_the_turn_over_and_is_recorded() {
        let mut game = load_go();

        pass(&mut game).unwrap();

        assert_eq!(game.current_player(), "WHITE");
        let last = game.history().last().unwrap();
        assert_eq!(last.action, "PASS");
        assert!(last.passed);
        assert_eq!(last.player, "BLACK");
        assert!(last.from.as_str().is_empty() && last.to.as_str().is_empty());
        assert!(phase_is_choosing(&game));
    }

    #[test]
    fn test_a_game_that_does_not_allow_passing_refuses_it() {
        let mut game = parse_game_spec("./specs/chess.json")
            .map(Game::from_spec)
            .unwrap();

        assert!(matches!(pass(&mut game), Err(GameError::NotAnOption)));
        assert_eq!(game.current_player(), "WHITE");
    }

    #[test]
    fn test_a_pass_cancels_a_piece_that_was_picked_up() {
        let mut game = stepping();
        put(&mut game, 0, 0, "BLACK");
        pick(&mut game, pos![0, 0]).unwrap();
        assert_eq!(chosen(&game), vec![Answer::Place(pos![0, 0])]);

        pass(&mut game).unwrap();

        assert!(phase_is_choosing(&game));
        assert_eq!(game.choice().start, None, "nothing stays picked");
        assert!(game.history().last().is_some_and(|record| record.passed));
    }

    #[test]
    fn test_a_pass_is_refused_while_a_promotion_is_pending() {
        // Chess with passing allowed, and WHITE's pawn on the last rank asking for a promotion.
        let mut spec: serde_json::Value =
            serde_json::from_str(include_str!("../../../specs/chess.json")).unwrap();
        spec["actions"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({ "name": "PASS", "do": [] }));
        let mut game = parse_game_spec_str(&spec.to_string())
            .map(Game::from_spec)
            .unwrap();
        game.state.pieces.clear();
        for (at, code, player) in [
            (pos![4, 0], "KING", "WHITE"),
            (pos![4, 7], "KING", "BLACK"),
            (pos![0, 6], "PAWN", "WHITE"),
        ] {
            game.state
                .pieces
                .insert(at, Piece::new(code.to_string(), player.to_string()));
        }
        pick(&mut game, pos![0, 6]).unwrap();
        support::go(&mut game, pos![0, 7]).unwrap();
        assert!(!promotion_options(&game).is_empty());

        let named = game.transition(GameTransition::Choose(Answer::Action("PASS".to_string())));
        assert!(matches!(named, Err(GameError::NotAnOption)));
        assert!(matches!(pass(&mut game), Err(GameError::CannotCancel)));
        assert!(
            !promotion_options(&game).is_empty(),
            "the promotion is still pending"
        );
        assert_eq!(game.current_player(), "WHITE");
    }

    #[test]
    fn test_a_pass_is_refused_once_the_game_is_over() {
        let mut game = load_go();
        pass(&mut game).unwrap();
        pass(&mut game).unwrap();
        assert!(matches!(game.state.phase, GamePhase::GameOver { .. }));

        assert!(matches!(pass(&mut game), Err(GameError::InvalidGamePhase)));
    }

    #[test]
    fn test_a_player_with_nothing_else_to_do_can_always_pass() {
        let mut game = load_go();
        for x in 0..9 {
            for y in 0..9 {
                put(&mut game, x, y, "WHITE");
            }
        }
        // Not a position that arises in play, but there is no empty point left at all.
        game.state.phase = choosing();

        assert!(game.any_legal_moves());
    }

    // -----------------------------------------------------------------------
    // Area scoring
    // -----------------------------------------------------------------------

    /// BLACK holds column 3, WHITE column 5, and column 4 between them touches both.
    fn two_walls() -> Game {
        let mut game = load_go();
        for y in 0..9 {
            put(&mut game, 3, y, "BLACK");
            put(&mut game, 5, y, "WHITE");
        }
        game
    }

    #[test]
    fn test_scores_count_stones_and_the_empty_points_only_one_side_touches() {
        let game = two_walls();

        let scores = game.scores().expect("the game scores by area");

        // 9 stones + the 27 points of columns 0 to 2, against 9 + 27 and the 7.5 bonus.
        assert_eq!(scores.get("BLACK").copied(), Some(36.0));
        assert_eq!(scores.get("WHITE").copied(), Some(43.5));
    }

    #[test]
    fn test_the_strip_between_the_walls_counts_for_nobody_until_it_holds_stones() {
        let mut game = two_walls();
        let before = game.scores().unwrap();
        assert_eq!(before.get("BLACK").copied(), Some(36.0));

        for y in 0..9 {
            put(&mut game, 4, y, "BLACK");
        }
        let after = game.scores().unwrap();

        // 18 stones, the 27 points to the left, and now the right side is WHITE's alone.
        assert_eq!(after.get("BLACK").copied(), Some(45.0));
        assert_eq!(after.get("WHITE").copied(), Some(9.0 + 27.0 + 7.5));
    }

    #[test]
    fn test_an_empty_board_gives_no_territory() {
        let game = load_go();

        let scores = game.scores().unwrap();

        assert_eq!(scores.get("BLACK").copied(), Some(0.0));
        assert_eq!(scores.get("WHITE").copied(), Some(7.5));
    }

    #[test]
    fn test_two_passes_in_a_row_end_the_game_and_the_higher_score_wins() {
        let mut game = two_walls();

        pass(&mut game).unwrap();
        assert!(phase_is_choosing(&game), "one pass does not end it");
        pass(&mut game).unwrap();

        assert_eq!(
            game.state.phase,
            GamePhase::GameOver {
                winner: Some("WHITE".to_string())
            }
        );
    }

    #[test]
    fn test_a_pass_in_between_moves_does_not_end_the_game() {
        let mut game = load_go();

        pass(&mut game).unwrap();
        place(&mut game, 4, 4).unwrap();
        pass(&mut game).unwrap();

        assert!(phase_is_choosing(&game));
    }

    #[test]
    fn test_equal_scores_are_a_draw() {
        let spec = std::fs::read_to_string("./specs/go.json")
            .unwrap()
            .replace("7.5", "0.0");
        let mut game = Game::from_spec(parse_game_spec_str(&spec).unwrap());
        put(&mut game, 0, 0, "BLACK");
        put(&mut game, 8, 8, "WHITE");
        // Neither wall is closed, so the one empty region touches both and counts for nobody.

        pass(&mut game).unwrap();
        pass(&mut game).unwrap();

        assert_eq!(game.state.phase, GamePhase::GameOver { winner: None });
    }

    #[test]
    fn test_without_territory_only_the_stones_are_counted() {
        let spec = std::fs::read_to_string("./specs/go.json").unwrap().replace(
            "{ \"term\": \"TERRITORY\", \"where\": { \"codes\": [\"STONE\"] } },",
            "",
        );
        let mut game = Game::from_spec(parse_game_spec_str(&spec).unwrap());
        assert_eq!(game.rules.score.len(), 2, "the territory term is gone");
        for y in 0..9 {
            put(&mut game, 3, y, "BLACK");
        }
        put(&mut game, 8, 8, "WHITE");

        let scores = game.scores().unwrap();

        assert_eq!(scores.get("BLACK").copied(), Some(9.0));
        assert_eq!(scores.get("WHITE").copied(), Some(8.5));
    }

    #[test]
    fn test_a_game_without_a_score_has_no_scores() {
        let game = parse_game_spec("./specs/chess.json")
            .map(Game::from_spec)
            .unwrap();

        assert!(game.scores().is_none());
    }

    #[test]
    fn test_the_game_can_be_saved_and_restored_in_the_middle() {
        let mut game = ko();
        place(&mut game, 2, 1).unwrap();
        let saved = game.save_state().unwrap();

        let mut restored = load_go();
        restored.restore_state(&saved).unwrap();

        assert!(!can_place(&mut restored, 1, 1), "the ko survives a restore");
    }

    // -----------------------------------------------------------------------
    // The same rules on pieces that move
    // -----------------------------------------------------------------------

    /// A 5x5 board on which a stone steps one point, or jumps two along a row. Captures follow
    /// the same rule as for stones that are placed.
    fn capture_by_stepping() -> Value {
        let moves: Vec<Value> = [[1, 0], [-1, 0], [0, 1], [0, -1], [2, 0], [-2, 0]]
            .iter()
            .enumerate()
            .map(|(id, step)| json!({ "id": id, "step": step, "do": onto_an_empty_square() }))
            .collect();
        json!({
            "name": "STEPS",
            "board": { "dimensions": [5, 5] },
            "players": [
                { "name": "BLACK", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] }
            ],
            "turns": { "order": ["BLACK", "WHITE"] },
            "actions": [move_action(), pass_action()],
            "every_action": [support::liberty_rule(&["MOVE"])],
            "pieces": [{ "code": "STONE", "name": "stone", "moves": moves }]
        })
    }

    fn stepping() -> Game {
        parse_game_spec_str(&capture_by_stepping().to_string())
            .map(Game::from_spec)
            .unwrap()
    }

    fn destinations(game: &mut Game, x: u8, y: u8) -> Vec<Position> {
        moves_from(game, pos![x, y])
    }

    #[test]
    fn test_a_stone_that_steps_into_the_last_liberty_captures() {
        let mut game = stepping();
        put(&mut game, 0, 0, "WHITE");
        put(&mut game, 1, 0, "BLACK");
        put(&mut game, 1, 1, "BLACK");

        pick(&mut game, pos![1, 1]).unwrap();
        support::go(&mut game, pos![0, 1]).unwrap();

        assert_eq!(owner(&game, 0, 0), None);
        assert_eq!(lost(&game, "WHITE"), 1);
    }

    #[test]
    fn test_a_stone_may_not_move_to_a_point_that_leaves_it_without_liberties() {
        // The jump from (2,0) to the corner would leave the stone between two white stones, and
        // the white stone at (1,0) keeps (2,0), the point just vacated, as a liberty.
        let mut game = stepping();
        put(&mut game, 1, 0, "WHITE");
        put(&mut game, 0, 1, "WHITE");
        put(&mut game, 1, 1, "BLACK");
        put(&mut game, 2, 0, "BLACK");

        let reachable = destinations(&mut game, 2, 0);

        assert!(!reachable.contains(&pos![0, 0]), "suicide is refused");
        assert!(reachable.contains(&pos![3, 0]), "other moves are untouched");
    }

    #[test]
    fn test_a_game_with_no_liberty_rule_lets_the_same_stone_jump_in() {
        let mut spec = capture_by_stepping();
        spec.as_object_mut().unwrap().remove("every_action");
        let mut game = parse_game_spec_str(&spec.to_string())
            .map(Game::from_spec)
            .unwrap();
        put(&mut game, 1, 0, "WHITE");
        put(&mut game, 0, 1, "WHITE");
        put(&mut game, 1, 1, "BLACK");
        put(&mut game, 2, 0, "BLACK");

        assert!(destinations(&mut game, 2, 0).contains(&pos![0, 0]));
    }

    // -----------------------------------------------------------------------
    // The bigger boards
    // -----------------------------------------------------------------------

    /// The three sizes are the same rules on boards of different sizes. Comparing them keeps a
    /// change to one file from being forgotten in the others.
    #[test]
    fn test_the_bigger_boards_differ_from_the_small_one_only_in_name_and_size() {
        let read = |file: &str| -> serde_json::Value {
            let text = std::fs::read_to_string(file).unwrap();
            serde_json::from_str(&text).unwrap()
        };
        let small = read("./specs/go.json");

        for (file, size) in [("./specs/go_13.json", 13), ("./specs/go_19.json", 19)] {
            let mut big = read(file);
            assert_eq!(big["board"]["dimensions"], serde_json::json!([size, size]));
            assert_eq!(big["name"], format!("GO {size}X{size}"));
            big["board"] = small["board"].clone();
            big["name"] = small["name"].clone();
            assert_eq!(big, small, "{file} has drifted from go.json");
        }
    }

    #[test]
    fn test_a_19_by_19_game_plays_and_captures() {
        let mut game = parse_game_spec("./specs/go_19.json")
            .map(Game::from_spec)
            .unwrap();
        put(&mut game, 18, 18, "WHITE");
        put(&mut game, 17, 18, "BLACK");

        place(&mut game, 18, 17).unwrap();

        assert_eq!(owner(&game, 18, 18), None, "the corner stone is taken");
    }

    // -----------------------------------------------------------------------
    // The rule as blocks
    // -----------------------------------------------------------------------

    #[test]
    fn go_carries_the_rule_the_other_tests_build_by_hand() {
        let spec = go_spec();
        let entry = &spec["every_action"][0];
        assert_eq!(entry["for"], json!(["STONE"]), "a pass is not named");
        let steps = entry["do"].as_array().unwrap();
        assert_eq!(&steps[..2], support::liberty_steps().as_slice());
        assert_eq!(
            steps[2],
            json!({ "test": "NOT", "of": { "test": "REPEATS" } })
        );
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn a_move_that_captures_first_then_has_a_liberty_is_legal() {
        // Playing the corner fills BLACK's last empty neighbour, but it takes both white
        // stones first, and that frees them.
        let mut game = load_go();
        for (x, y) in [(1, 0), (0, 1)] {
            put(&mut game, x, y, "WHITE");
        }
        for (x, y) in [(2, 0), (1, 1), (0, 2)] {
            put(&mut game, x, y, "BLACK");
        }
        assert!(can_place(&mut game, 0, 0));
        place(&mut game, 0, 0).unwrap();
        assert_eq!(owner(&game, 0, 0), Some("BLACK"));
        assert_eq!(owner(&game, 1, 0), None);
        assert_eq!(owner(&game, 0, 1), None);
    }

    #[test]
    fn filling_ones_own_last_liberty_without_capturing_is_refused() {
        // Both white stones keep a liberty of their own, so nothing is taken.
        let mut game = load_go();
        for (x, y) in [(1, 0), (0, 1)] {
            put(&mut game, x, y, "WHITE");
        }
        put(&mut game, 1, 1, "BLACK");
        assert!(
            !can_place(&mut game, 0, 0),
            "a suicide that captures nothing"
        );
        assert!(can_place(&mut game, 2, 2), "other points are fine");
    }

    #[test]
    fn a_stone_move_that_recreates_a_position_is_refused_but_a_pass_is_not() {
        let mut game = ko();
        place(&mut game, 2, 1).unwrap(); // BLACK takes the white stone
        assert!(!can_place(&mut game, 1, 1), "the retake repeats a position");
        assert!(
            pass(&mut game).is_ok(),
            "a pass is never refused for repeating"
        );
    }

    #[test]
    fn a_go_capture_carries_the_capture_mark_in_the_history() {
        let mut game = ko();
        place(&mut game, 2, 1).unwrap(); // BLACK takes the white stone at (1,1)
        let record = game.history().last().expect("the move is recorded").clone();
        assert!(record.captures, "the stone taken is an enemy piece removed");
        place(&mut game, 7, 7).unwrap();
        assert!(
            !game.history().last().unwrap().captures,
            "a quiet stone is not a capture"
        );
    }
}
