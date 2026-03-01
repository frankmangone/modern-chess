#[cfg(test)]
mod tests {
    use crate::logic::{Game, GameTransition, Piece, PieceState};
    use crate::specs::parse_game_spec;

    fn load_game() -> Game {
        parse_game_spec("./src/tests/conditions/spec.json")
            .map(Game::from_spec)
            .expect("Failed to load conditions test spec")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, code: &str) {
        game.state.pieces.insert(pos, Piece::new(code.to_string(), "WHITE".to_string()));
    }

    // -------------------------------------------------------------------------
    // CHECK_STATE
    // -------------------------------------------------------------------------

    /// STATE_MOVER at [0,0] can only step to [0,1] when the piece at [1,0]
    /// (relative offset [1,0]) carries the "TAGGED" state flag.
    #[test]
    fn test_check_state_blocks_move_when_flag_absent() {
        let mut game = load_game();
        insert(&mut game, vec![0, 0], "STATE_MOVER");
        insert(&mut game, vec![1, 0], "DUMMY"); // no TAGGED flag

        game.transition(GameTransition::CalculateMoves { position: vec![0, 0] }).unwrap();
        assert!(
            game.state.available_moves.is_none(),
            "No moves expected: DUMMY at [1,0] has no TAGGED state"
        );
    }

    #[test]
    fn test_check_state_allows_move_when_flag_present() {
        let mut game = load_game();
        insert(&mut game, vec![0, 0], "STATE_MOVER");
        insert(&mut game, vec![1, 0], "DUMMY");

        // Set TAGGED flag on the DUMMY piece.
        game.state.pieces.get_mut(&vec![1u8, 0u8]).unwrap()
            .state.insert("TAGGED".to_string(), PieceState::Blank);

        game.transition(GameTransition::CalculateMoves { position: vec![0, 0] }).unwrap();
        assert!(
            game.state.available_moves.as_ref().unwrap().contains_key(&vec![0u8, 1u8]),
            "Move to [0,1] should be available when DUMMY has TAGGED state"
        );
    }

    #[test]
    fn test_check_state_blocks_move_after_flag_expires() {
        let mut game = load_game();
        insert(&mut game, vec![0, 0], "STATE_MOVER");
        insert(&mut game, vec![1, 0], "DUMMY");

        game.state.pieces.get_mut(&vec![1u8, 0u8]).unwrap()
            .state.insert("TAGGED".to_string(), PieceState::Uint(1));

        // Two ticks expire the flag (Uint(1) → Uint(0) → removed).
        game.next_turn();
        game.next_turn();

        game.transition(GameTransition::CalculateMoves { position: vec![0, 0] }).unwrap();
        assert!(
            game.state.available_moves.is_none(),
            "No moves expected: TAGGED flag should have expired"
        );
    }

    // -------------------------------------------------------------------------
    // PIECE_FIRST_MOVE
    // -------------------------------------------------------------------------

    /// FIRST_MOVE_JUMPER at [0,0] can only jump to [0,2] when the piece at [0,1]
    /// (relative offset [0,1]) has never moved.
    #[test]
    fn test_piece_first_move_allows_jump_when_target_piece_is_fresh() {
        let mut game = load_game();
        insert(&mut game, vec![0, 0], "FIRST_MOVE_JUMPER");
        insert(&mut game, vec![0, 1], "DUMMY"); // total_moves == 0

        game.transition(GameTransition::CalculateMoves { position: vec![0, 0] }).unwrap();
        assert!(
            game.state.available_moves.as_ref().unwrap().contains_key(&vec![0u8, 2u8]),
            "Jump to [0,2] should be available: DUMMY at [0,1] has never moved"
        );
    }

    #[test]
    fn test_piece_first_move_blocks_jump_when_target_piece_has_moved() {
        let mut game = load_game();
        insert(&mut game, vec![0, 0], "FIRST_MOVE_JUMPER");
        insert(&mut game, vec![0, 1], "DUMMY");

        game.state.pieces.get_mut(&vec![0u8, 1u8]).unwrap().total_moves = 1;

        game.transition(GameTransition::CalculateMoves { position: vec![0, 0] }).unwrap();
        assert!(
            game.state.available_moves.is_none(),
            "Jump should be blocked: DUMMY at [0,1] has already moved"
        );
    }

    #[test]
    fn test_piece_first_move_blocks_when_no_piece_at_position() {
        let mut game = load_game();
        insert(&mut game, vec![0, 0], "FIRST_MOVE_JUMPER");
        // No DUMMY at [0,1].

        game.transition(GameTransition::CalculateMoves { position: vec![0, 0] }).unwrap();
        assert!(
            game.state.available_moves.is_none(),
            "Jump should be blocked: no piece at [0,1] to verify first-move status"
        );
    }

    // -------------------------------------------------------------------------
    // PATH_EMPTY
    // -------------------------------------------------------------------------

    /// PATH_JUMPER at [0,0] can jump to [0,2] only when [0,1] is empty.
    #[test]
    fn test_path_empty_allows_jump_when_path_is_clear() {
        let mut game = load_game();
        insert(&mut game, vec![0, 0], "PATH_JUMPER");
        // [0,1] is empty — path is clear.

        game.transition(GameTransition::CalculateMoves { position: vec![0, 0] }).unwrap();
        assert!(
            game.state.available_moves.as_ref().unwrap().contains_key(&vec![0u8, 2u8]),
            "Jump to [0,2] should be available: path through [0,1] is clear"
        );
    }

    #[test]
    fn test_path_empty_blocks_jump_when_path_is_occupied() {
        let mut game = load_game();
        insert(&mut game, vec![0, 0], "PATH_JUMPER");
        insert(&mut game, vec![0, 1], "DUMMY"); // blocks the path

        game.transition(GameTransition::CalculateMoves { position: vec![0, 0] }).unwrap();
        assert!(
            game.state.available_moves.is_none(),
            "Jump should be blocked: DUMMY at [0,1] is in the path"
        );
    }
}
