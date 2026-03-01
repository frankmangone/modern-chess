#[cfg(test)]
mod tests {
    use crate::logic::{Game, GameTransition, Piece, PieceState};
    use crate::specs::parse_game_spec;

    fn load_game() -> Game {
        parse_game_spec("./src/tests/side_effects/spec.json")
            .map(Game::from_spec)
            .expect("Failed to load side_effects test spec")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, code: &str, player: &str) {
        game.state.pieces.insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    // -------------------------------------------------------------------------
    // SET_STATE
    // -------------------------------------------------------------------------

    /// DOUBLE_PUSHER (WHITE) at [3,1] executes a double push to [3,3].
    /// The SET_STATE side effect must attach an "EN_PASSANT" flag to the moved piece.
    /// After execute_move, next_turn() ticks the flag from Uint(1) to Uint(0), but
    /// it should still be present in the piece's state map.
    #[test]
    fn test_set_state_attaches_en_passant_flag() {
        let mut game = load_game();
        insert(&mut game, vec![3, 1], "DOUBLE_PUSHER", "WHITE");

        game.transition(GameTransition::CalculateMoves { position: vec![3, 1] }).unwrap();
        assert!(
            game.state.available_moves.as_ref().unwrap().contains_key(&vec![3u8, 3u8]),
            "Double push to [3,3] should be available"
        );

        game.transition(GameTransition::ExecuteMove { position: vec![3, 3] }).unwrap();

        let piece = game.state.pieces.get(&vec![3u8, 3u8])
            .expect("DOUBLE_PUSHER should be at [3,3] after the move");
        assert!(
            piece.state.contains_key("EN_PASSANT"),
            "EN_PASSANT flag should be set on the piece after a double push"
        );
    }

    // -------------------------------------------------------------------------
    // CAPTURE side effect (en passant)
    // -------------------------------------------------------------------------

    /// WHITE EN_PASSANT_CAPTOR at [3,5], BLACK DOUBLE_PUSHER at [4,5] with EN_PASSANT flag.
    /// The captor's EMPTY→MOVE action (step [1,1]) is gated by CHECK_STATE EN_PASSANT at [4,5],
    /// and fires a CAPTURE side effect that clears [4,5].
    /// After executing the move to [4,6]:
    ///   - WHITE piece lands at [4,6]
    ///   - [3,5] (source) is cleared
    ///   - [4,5] (en-passant pawn) is cleared
    #[test]
    fn test_en_passant_capture_removes_captured_pawn() {
        let mut game = load_game();
        insert(&mut game, vec![3, 5], "EN_PASSANT_CAPTOR", "WHITE");
        insert(&mut game, vec![4, 5], "DOUBLE_PUSHER", "BLACK");

        // Manually set EN_PASSANT flag (simulates the flag set last turn and ticked once).
        game.state.pieces.get_mut(&vec![4u8, 5u8]).unwrap()
            .state.insert("EN_PASSANT".to_string(), PieceState::Uint(0));

        game.transition(GameTransition::CalculateMoves { position: vec![3, 5] }).unwrap();
        assert!(
            game.state.available_moves.as_ref().unwrap().contains_key(&vec![4u8, 6u8]),
            "En passant move to [4,6] should be available when neighbour has EN_PASSANT flag"
        );

        game.transition(GameTransition::ExecuteMove { position: vec![4, 6] }).unwrap();

        assert!(
            game.state.pieces.get(&vec![4u8, 6u8]).is_some(),
            "WHITE piece should be at [4,6] after en passant"
        );
        assert_eq!(
            game.state.pieces.get(&vec![4u8, 6u8]).unwrap().player,
            "WHITE",
            "Piece at [4,6] should belong to WHITE"
        );
        assert!(
            game.state.pieces.get(&vec![4u8, 5u8]).is_none(),
            "BLACK pawn at [4,5] should be removed by the CAPTURE side effect"
        );
        assert!(
            game.state.pieces.get(&vec![3u8, 5u8]).is_none(),
            "Source square [3,5] should be cleared"
        );
    }

    /// Without the EN_PASSANT flag, the EMPTY→MOVE action's condition fails and the diagonal
    /// move to an empty square should NOT be available.
    #[test]
    fn test_en_passant_blocked_when_flag_absent() {
        let mut game = load_game();
        insert(&mut game, vec![3, 5], "EN_PASSANT_CAPTOR", "WHITE");
        insert(&mut game, vec![4, 5], "DOUBLE_PUSHER", "BLACK");
        // No EN_PASSANT flag set.

        game.transition(GameTransition::CalculateMoves { position: vec![3, 5] }).unwrap();
        let moves = game.state.available_moves.as_ref();
        // [4,6] should not be available (empty square, no EN_PASSANT flag on [4,5]).
        // [4,6] ENEMY capture would need an enemy piece there — it's empty, so ENEMY action won't fire.
        assert!(
            moves.map_or(true, |m| !m.contains_key(&vec![4u8, 6u8])),
            "[4,6] should not be available: EN_PASSANT flag is absent"
        );
    }

    // -------------------------------------------------------------------------
    // MOVE side effect (castling-style rook repositioning)
    // -------------------------------------------------------------------------

    /// WHITE CASTLER at [0,0], WHITE ROOK_PARTNER at [3,0].
    /// CASTLER moves 2 squares right to [2,0] (FIRST_MOVE + PATH_EMPTY satisfied).
    /// The MOVE side effect repositions the ROOK_PARTNER from [3,0] to [1,0].
    #[test]
    fn test_castling_side_effect_moves_rook_partner() {
        let mut game = load_game();
        insert(&mut game, vec![0, 0], "CASTLER", "WHITE");
        insert(&mut game, vec![3, 0], "ROOK_PARTNER", "WHITE");

        game.transition(GameTransition::CalculateMoves { position: vec![0, 0] }).unwrap();
        assert!(
            game.state.available_moves.as_ref().unwrap().contains_key(&vec![2u8, 0u8]),
            "Castle to [2,0] should be available"
        );

        game.transition(GameTransition::ExecuteMove { position: vec![2, 0] }).unwrap();

        assert!(
            game.state.pieces.get(&vec![2u8, 0u8]).is_some(),
            "CASTLER should be at [2,0]"
        );
        assert!(
            game.state.pieces.get(&vec![0u8, 0u8]).is_none(),
            "Source [0,0] should be cleared"
        );
        assert!(
            game.state.pieces.get(&vec![1u8, 0u8]).is_some(),
            "ROOK_PARTNER should have moved to [1,0]"
        );
        assert_eq!(
            game.state.pieces.get(&vec![1u8, 0u8]).unwrap().code,
            "ROOK_PARTNER",
            "Piece at [1,0] should be the ROOK_PARTNER"
        );
        assert!(
            game.state.pieces.get(&vec![3u8, 0u8]).is_none(),
            "ROOK_PARTNER's original square [3,0] should be cleared"
        );
    }
}
