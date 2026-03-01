#[cfg(test)]
mod tests {
    use crate::logic::{Game, PieceState};
    use crate::specs::{parse_game_spec, GameSpecError};

    // Load a minimal single-player spec so we can freely call next_turn()
    // without worrying about the turn order.
    fn load_game() -> Result<Game, GameSpecError> {
        let spec = parse_game_spec("./src/tests/transformation/spec.json")?;
        Ok(Game::from_spec(spec))
    }

    // Convenience: position of the first pawn in the transformation spec.
    fn pawn_pos() -> Vec<u8> { vec![0, 0] }

    // ---------------------------------------------------------------------------
    // tick_state_flags unit tests (on Piece directly)
    // ---------------------------------------------------------------------------

    #[test]
    fn test_uint_flag_decrements_each_tick() {
        let mut game = load_game().unwrap();
        game.state.pieces.get_mut(&pawn_pos()).unwrap()
            .state.insert("FLAG".to_string(), PieceState::Uint(2));

        game.next_turn();
        assert!(matches!(
            game.state.pieces.get(&pawn_pos()).unwrap().state.get("FLAG"),
            Some(PieceState::Uint(1))
        ), "After 1 tick Uint(2) should become Uint(1)");

        game.next_turn();
        assert!(matches!(
            game.state.pieces.get(&pawn_pos()).unwrap().state.get("FLAG"),
            Some(PieceState::Uint(0))
        ), "After 2 ticks Uint(2) should become Uint(0) and still be present");

        game.next_turn();
        assert!(
            !game.state.pieces.get(&pawn_pos()).unwrap().state.contains_key("FLAG"),
            "After 3 ticks Uint(2) should be removed"
        );
    }

    #[test]
    fn test_duration_one_flag_available_for_one_turn_then_gone() {
        let mut game = load_game().unwrap();
        game.state.pieces.get_mut(&pawn_pos()).unwrap()
            .state.insert("EN_PASSANT".to_string(), PieceState::Uint(1));

        // First next_turn: flag ticks from 1 → 0 but stays present.
        game.next_turn();
        assert!(
            game.state.pieces.get(&pawn_pos()).unwrap().state.contains_key("EN_PASSANT"),
            "EN_PASSANT should still be present after the first tick (Uint(0))"
        );

        // Second next_turn: flag was at 0 → removed.
        game.next_turn();
        assert!(
            !game.state.pieces.get(&pawn_pos()).unwrap().state.contains_key("EN_PASSANT"),
            "EN_PASSANT should be gone after the second tick"
        );
    }

    #[test]
    fn test_blank_flag_is_permanent() {
        let mut game = load_game().unwrap();
        game.state.pieces.get_mut(&pawn_pos()).unwrap()
            .state.insert("PERMANENT".to_string(), PieceState::Blank);

        for _ in 0..5 {
            game.next_turn();
        }

        assert!(
            game.state.pieces.get(&pawn_pos()).unwrap().state.contains_key("PERMANENT"),
            "Blank flags should never be removed by the ticker"
        );
    }

    #[test]
    fn test_string_flag_is_permanent() {
        let mut game = load_game().unwrap();
        game.state.pieces.get_mut(&pawn_pos()).unwrap()
            .state.insert("LABEL".to_string(), PieceState::String("some_value".to_string()));

        for _ in 0..5 {
            game.next_turn();
        }

        assert!(
            game.state.pieces.get(&pawn_pos()).unwrap().state.contains_key("LABEL"),
            "String flags should never be removed by the ticker"
        );
    }

    #[test]
    fn test_multiple_flags_expire_independently() {
        let mut game = load_game().unwrap();
        let piece = game.state.pieces.get_mut(&pawn_pos()).unwrap();
        piece.state.insert("SHORT".to_string(), PieceState::Uint(1));
        piece.state.insert("LONG".to_string(),  PieceState::Uint(3));
        piece.state.insert("PERM".to_string(),  PieceState::Blank);

        game.next_turn(); // SHORT→0, LONG→2
        game.next_turn(); // SHORT removed, LONG→1

        let state = &game.state.pieces.get(&pawn_pos()).unwrap().state;
        assert!(!state.contains_key("SHORT"), "SHORT should be gone after 2 ticks");
        assert!(state.contains_key("LONG"),   "LONG should still be present after 2 ticks");
        assert!(state.contains_key("PERM"),   "PERM should still be present");

        game.next_turn(); // LONG→0
        game.next_turn(); // LONG removed

        let state = &game.state.pieces.get(&pawn_pos()).unwrap().state;
        assert!(!state.contains_key("LONG"), "LONG should be gone after 4 ticks");
        assert!(state.contains_key("PERM"),  "PERM should still be present");
    }
}
