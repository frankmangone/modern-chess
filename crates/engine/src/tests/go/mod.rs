//! Placing stones from a supply, capturing groups with no liberties, passing and area scoring,
//! exercised through a 9x9 Go spec. None of the rules below knows about Go: each is a switch in
//! the spec (`unlimited_supply`, `liberty_capture`, `forbid_repeated_positions`, `allow_pass`
//! and the `AREA_SCORE` win condition).

#[cfg(test)]
mod tests {
    use crate::logic::{Game, GameError, GamePhase, GameTransition, Piece};
    use crate::specs::{parse_game_spec, parse_game_spec_str};

    const STONE: &str = "STONE";

    fn load_go() -> Game {
        parse_game_spec("./specs/go.json")
            .map(Game::from_spec)
            .expect("Failed to load go spec")
    }

    fn put(game: &mut Game, x: u8, y: u8, player: &str) {
        game.state.pieces.insert(
            vec![x, y],
            Piece::new(STONE.to_string(), player.to_string()),
        );
    }

    fn owner(game: &Game, x: u8, y: u8) -> Option<&str> {
        game.state
            .pieces
            .get(&vec![x, y])
            .map(|p| p.player.as_str())
    }

    fn place(game: &mut Game, x: u8, y: u8) -> Result<(), GameError> {
        game.transition(GameTransition::CalculateDrops {
            piece_code: STONE.to_string(),
        })?;
        game.transition(GameTransition::ExecuteDrop {
            position: vec![x, y],
        })
    }

    fn can_place(game: &mut Game, x: u8, y: u8) -> bool {
        game.transition(GameTransition::CalculateDrops {
            piece_code: STONE.to_string(),
        })
        .expect("the supply is never empty");
        let legal = game
            .state
            .available_moves
            .as_ref()
            .is_some_and(|moves| moves.contains_key(&vec![x, y]));
        game.state.available_moves = None;
        game.state.phase = GamePhase::Idle;
        legal
    }

    fn pass(game: &mut Game) -> Result<(), GameError> {
        game.transition(GameTransition::Pass)
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

        game.transition(GameTransition::CalculateDrops {
            piece_code: STONE.to_string(),
        })
        .unwrap();
        let offered = game.state.available_moves.as_ref().unwrap();

        assert_eq!(offered.len(), 80);
        assert!(!offered.contains_key(&vec![0, 0]));
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
        let result = game.transition(GameTransition::CalculateDrops {
            piece_code: "PAWN".to_string(),
        });
        assert!(matches!(result, Err(GameError::PieceNotInHand)));
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

        game.transition(GameTransition::CalculateDrops {
            piece_code: STONE.to_string(),
        })
        .unwrap();
        let result = game.transition(GameTransition::ExecuteDrop {
            position: vec![0, 0],
        });

        assert!(matches!(result, Err(GameError::InvalidMove)));
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

    #[test]
    fn test_suicide_can_be_allowed_and_then_removes_the_stones_just_placed() {
        let spec = std::fs::read_to_string("./specs/go.json").unwrap().replace(
            "\"liberty_capture\": { \"pieces\": [\"STONE\"] }",
            "\"liberty_capture\": { \"pieces\": [\"STONE\"], \"allow_suicide\": true }",
        );
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
        let spec = std::fs::read_to_string("./specs/go.json").unwrap().replace(
            "\"forbid_repeated_positions\": true",
            "\"forbid_repeated_positions\": false",
        );
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
        assert_eq!(last.player, "BLACK");
        assert!(last.from.is_empty() && last.to.is_empty());
        assert_eq!(game.state.phase, GamePhase::Idle);
    }

    #[test]
    fn test_a_game_that_does_not_allow_passing_refuses_it() {
        let mut game = parse_game_spec("./specs/chess.json")
            .map(Game::from_spec)
            .unwrap();

        assert!(matches!(pass(&mut game), Err(GameError::PassNotAllowed)));
        assert_eq!(game.current_player(), "WHITE");
    }

    #[test]
    fn test_a_pass_cancels_a_piece_that_was_picked_up() {
        let mut game = stepping();
        put(&mut game, 0, 0, "BLACK");
        game.transition(GameTransition::CalculateMoves {
            position: vec![0, 0],
        })
        .unwrap();
        assert!(matches!(game.state.phase, GamePhase::Moving { .. }));

        pass(&mut game).unwrap();

        assert_eq!(game.state.phase, GamePhase::Idle);
        assert!(game.state.available_moves.is_none());
    }

    #[test]
    fn test_a_pass_is_refused_while_a_promotion_is_pending() {
        let mut game = load_go();
        game.state.phase = GamePhase::Transforming {
            position: vec![0, 0],
            options: vec![STONE.to_string()],
        };

        assert!(matches!(pass(&mut game), Err(GameError::InvalidGamePhase)));
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
        game.state.phase = GamePhase::Idle;

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

        let scores = game.area_scores().expect("the game scores by area");

        // 9 stones + the 27 points of columns 0 to 2, against 9 + 27 and the 7.5 bonus.
        assert_eq!(scores.get("BLACK").copied(), Some(36.0));
        assert_eq!(scores.get("WHITE").copied(), Some(43.5));
    }

    #[test]
    fn test_the_strip_between_the_walls_counts_for_nobody_until_it_holds_stones() {
        let mut game = two_walls();
        let before = game.area_scores().unwrap();
        assert_eq!(before.get("BLACK").copied(), Some(36.0));

        for y in 0..9 {
            put(&mut game, 4, y, "BLACK");
        }
        let after = game.area_scores().unwrap();

        // 18 stones, the 27 points to the left, and now the right side is WHITE's alone.
        assert_eq!(after.get("BLACK").copied(), Some(45.0));
        assert_eq!(after.get("WHITE").copied(), Some(9.0 + 27.0 + 7.5));
    }

    #[test]
    fn test_an_empty_board_gives_no_territory() {
        let game = load_go();

        let scores = game.area_scores().unwrap();

        assert_eq!(scores.get("BLACK").copied(), Some(0.0));
        assert_eq!(scores.get("WHITE").copied(), Some(7.5));
    }

    #[test]
    fn test_two_passes_in_a_row_end_the_game_and_the_higher_score_wins() {
        let mut game = two_walls();

        pass(&mut game).unwrap();
        assert_eq!(
            game.state.phase,
            GamePhase::Idle,
            "one pass does not end it"
        );
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

        assert_eq!(game.state.phase, GamePhase::Idle);
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
        let spec = std::fs::read_to_string("./specs/go.json")
            .unwrap()
            .replace("\"territory\": true", "\"territory\": false");
        let mut game = Game::from_spec(parse_game_spec_str(&spec).unwrap());
        for y in 0..9 {
            put(&mut game, 3, y, "BLACK");
        }
        put(&mut game, 8, 8, "WHITE");

        let scores = game.area_scores().unwrap();

        assert_eq!(scores.get("BLACK").copied(), Some(9.0));
        assert_eq!(scores.get("WHITE").copied(), Some(8.5));
    }

    #[test]
    fn test_a_game_without_an_area_score_has_no_scores() {
        let game = parse_game_spec("./specs/chess.json")
            .map(Game::from_spec)
            .unwrap();

        assert!(game.area_scores().is_none());
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
    const CAPTURE_BY_STEPPING: &str = r#"{
        "name": "STEPS",
        "board": { "dimensions": [5, 5] },
        "players": [
          { "name": "BLACK", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
          { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] }
        ],
        "turns": { "order": ["BLACK", "WHITE"] },
        "allow_pass": true,
        "liberty_capture": { "pieces": ["STONE"] },
        "pieces": [{
          "code": "STONE",
          "name": "stone",
          "moves": [
            { "id": 0, "step": [1, 0],  "actions": [{ "state": "EMPTY", "action": "MOVE" }] },
            { "id": 1, "step": [-1, 0], "actions": [{ "state": "EMPTY", "action": "MOVE" }] },
            { "id": 2, "step": [0, 1],  "actions": [{ "state": "EMPTY", "action": "MOVE" }] },
            { "id": 3, "step": [0, -1], "actions": [{ "state": "EMPTY", "action": "MOVE" }] },
            { "id": 4, "step": [2, 0],  "actions": [{ "state": "EMPTY", "action": "MOVE" }] },
            { "id": 5, "step": [-2, 0], "actions": [{ "state": "EMPTY", "action": "MOVE" }] }
          ]
        }]
    }"#;

    fn stepping() -> Game {
        parse_game_spec_str(CAPTURE_BY_STEPPING)
            .map(Game::from_spec)
            .unwrap()
    }

    fn destinations(game: &mut Game, x: u8, y: u8) -> Vec<Vec<u8>> {
        game.transition(GameTransition::CalculateMoves {
            position: vec![x, y],
        })
        .unwrap();
        let mut found: Vec<Vec<u8>> = game
            .state
            .available_moves
            .as_ref()
            .map(|moves| moves.keys().cloned().collect())
            .unwrap_or_default();
        found.sort();
        found
    }

    #[test]
    fn test_a_stone_that_steps_into_the_last_liberty_captures() {
        let mut game = stepping();
        put(&mut game, 0, 0, "WHITE");
        put(&mut game, 1, 0, "BLACK");
        put(&mut game, 1, 1, "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![1, 1],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![0, 1],
        })
        .unwrap();

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

        assert!(!reachable.contains(&vec![0, 0]), "suicide is refused");
        assert!(reachable.contains(&vec![3, 0]), "other moves are untouched");
    }

    #[test]
    fn test_a_game_with_no_liberty_rule_lets_the_same_stone_jump_in() {
        let spec =
            CAPTURE_BY_STEPPING.replace("\"liberty_capture\": { \"pieces\": [\"STONE\"] },", "");
        let mut game = parse_game_spec_str(&spec).map(Game::from_spec).unwrap();
        put(&mut game, 1, 0, "WHITE");
        put(&mut game, 0, 1, "WHITE");
        put(&mut game, 1, 1, "BLACK");
        put(&mut game, 2, 0, "BLACK");

        assert!(destinations(&mut game, 2, 0).contains(&vec![0, 0]));
    }
}
