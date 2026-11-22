#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Game, Piece, PieceState};
    use crate::shared::Position;
    use crate::specs::{parse_game_spec, parse_game_spec_str, GameSpecError};
    use crate::tests::support::{pick, targets};

    fn load_game() -> Game {
        parse_game_spec("./src/tests/conditions/spec.json")
            .map(Game::from_spec)
            .expect("Failed to load conditions test spec")
    }

    fn insert(game: &mut Game, pos: Position, code: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), "WHITE".to_string()));
    }

    fn has_no_legal_moves(game: &Game) -> bool {
        targets(game).is_empty()
    }

    // -------------------------------------------------------------------------
    // A COUNT where the piece carries a state flag (CHECK_STATE before tests)
    // -------------------------------------------------------------------------

    /// STATE_MOVER at [0,0] can only step to [0,1] when the piece at [1,0]
    /// (relative offset [1,0]) carries the "TAGGED" state flag.
    #[test]
    fn test_check_state_blocks_move_when_flag_absent() {
        let mut game = load_game();
        insert(&mut game, pos![0, 0], "STATE_MOVER");
        insert(&mut game, pos![1, 0], "DUMMY"); // no TAGGED flag

        pick(&mut game, pos![0, 0]).unwrap();
        assert!(
            has_no_legal_moves(&game),
            "No moves expected: DUMMY at [1,0] has no TAGGED state"
        );
    }

    #[test]
    fn test_check_state_allows_move_when_flag_present() {
        let mut game = load_game();
        insert(&mut game, pos![0, 0], "STATE_MOVER");
        insert(&mut game, pos![1, 0], "DUMMY");

        // Set TAGGED flag on the DUMMY piece.
        game.state
            .pieces
            .get_mut(&pos![1u8, 0u8])
            .unwrap()
            .state
            .insert("TAGGED".to_string(), PieceState::Blank);

        pick(&mut game, pos![0, 0]).unwrap();
        assert!(
            targets(&game).contains(&pos![0u8, 1u8]),
            "Move to [0,1] should be available when DUMMY has TAGGED state"
        );
    }

    #[test]
    fn test_check_state_blocks_move_after_flag_expires() {
        let mut game = load_game();
        insert(&mut game, pos![0, 0], "STATE_MOVER");
        insert(&mut game, pos![1, 0], "DUMMY");

        game.state
            .pieces
            .get_mut(&pos![1u8, 0u8])
            .unwrap()
            .state
            .insert("TAGGED".to_string(), PieceState::Uint(1));

        // Two ticks expire the flag (Uint(1) → Uint(0) → removed).
        game.next_turn();
        game.next_turn();

        pick(&mut game, pos![0, 0]).unwrap();
        assert!(
            has_no_legal_moves(&game),
            "No moves expected: TAGGED flag should have expired"
        );
    }

    // -------------------------------------------------------------------------
    // A COUNT where the piece has not moved (PIECE_FIRST_MOVE before tests)
    // -------------------------------------------------------------------------

    /// FIRST_MOVE_JUMPER at [0,0] can only jump to [0,2] when the piece at [0,1]
    /// (relative offset [0,1]) has never moved.
    #[test]
    fn test_piece_first_move_allows_jump_when_target_piece_is_fresh() {
        let mut game = load_game();
        insert(&mut game, pos![0, 0], "FIRST_MOVE_JUMPER");
        insert(&mut game, pos![0, 1], "DUMMY"); // total_moves == 0

        pick(&mut game, pos![0, 0]).unwrap();
        assert!(
            targets(&game).contains(&pos![0u8, 2u8]),
            "Jump to [0,2] should be available: DUMMY at [0,1] has never moved"
        );
    }

    #[test]
    fn test_piece_first_move_blocks_jump_when_target_piece_has_moved() {
        let mut game = load_game();
        insert(&mut game, pos![0, 0], "FIRST_MOVE_JUMPER");
        insert(&mut game, pos![0, 1], "DUMMY");

        game.state
            .pieces
            .get_mut(&pos![0u8, 1u8])
            .unwrap()
            .total_moves = 1;

        pick(&mut game, pos![0, 0]).unwrap();
        assert!(
            has_no_legal_moves(&game),
            "Jump should be blocked: DUMMY at [0,1] has already moved"
        );
    }

    #[test]
    fn test_piece_first_move_blocks_when_no_piece_at_position() {
        let mut game = load_game();
        insert(&mut game, pos![0, 0], "FIRST_MOVE_JUMPER");
        // No DUMMY at [0,1].

        pick(&mut game, pos![0, 0]).unwrap();
        assert!(
            has_no_legal_moves(&game),
            "Jump should be blocked: no piece at [0,1] to verify first-move status"
        );
    }

    // -------------------------------------------------------------------------
    // A COUNT of at most none on the PATH to the target (PATH_EMPTY before tests)
    // -------------------------------------------------------------------------

    /// PATH_JUMPER at [0,0] can jump to [0,2] only when [0,1] is empty.
    #[test]
    fn test_path_empty_allows_jump_when_path_is_clear() {
        let mut game = load_game();
        insert(&mut game, pos![0, 0], "PATH_JUMPER");
        // [0,1] is empty — path is clear.

        pick(&mut game, pos![0, 0]).unwrap();
        assert!(
            targets(&game).contains(&pos![0u8, 2u8]),
            "Jump to [0,2] should be available: path through [0,1] is clear"
        );
    }

    #[test]
    fn test_path_empty_blocks_jump_when_path_is_occupied() {
        let mut game = load_game();
        insert(&mut game, pos![0, 0], "PATH_JUMPER");
        insert(&mut game, pos![0, 1], "DUMMY"); // blocks the path

        pick(&mut game, pos![0, 0]).unwrap();
        assert!(
            has_no_legal_moves(&game),
            "Jump should be blocked: DUMMY at [0,1] is in the path"
        );
    }

    // -------------------------------------------------------------------------
    // UNKNOWN TEST
    // A test the engine does not know is refused when the spec is read, rather than read as
    // false: an engine that does not recognise a rule must say so, not quietly drop the move.
    // -------------------------------------------------------------------------

    #[test]
    fn test_unknown_test_is_refused() {
        let mut spec: Value = serde_json::from_str(
            &std::fs::read_to_string("./src/tests/conditions/spec.json").expect("the spec"),
        )
        .expect("the spec is JSON");
        spec["pieces"][0]["moves"][0]["do"]
            .as_array_mut()
            .expect("steps")
            .insert(0, json!({ "test": "SOME_UNKNOWN_TEST" }));

        let error = parse_game_spec_str(&spec.to_string()).expect_err("an unknown test is refused");
        assert!(
            matches!(error, GameSpecError::JsonError(_)),
            "refused when read: {error}"
        );
        assert!(
            error.to_string().contains("SOME_UNKNOWN_TEST"),
            "the refusal names the test: {error}"
        );
    }
}
