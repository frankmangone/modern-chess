#[cfg(test)]
mod tests {
    use crate::logic::{Game, Piece, PieceState};
    use crate::shared::Position;
    use crate::specs::{parse_game_spec, parse_game_spec_str};
    use crate::tests::support::{go, pick, targets};

    fn load_game() -> Game {
        parse_game_spec("./src/tests/side_effects/spec.json")
            .map(Game::from_spec)
            .expect("Failed to load side_effects test spec")
    }

    fn insert(game: &mut Game, pos: Position, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    // -------------------------------------------------------------------------
    // SET_FLAG
    // -------------------------------------------------------------------------

    /// DOUBLE_PUSHER (WHITE) at [3,1] executes a double push to [3,3].
    /// The SET_FLAG effect must attach an "EN_PASSANT" flag to the moved piece.
    /// After execute_move, next_turn() ticks the flag from Uint(1) to Uint(0), but
    /// it should still be present in the piece's state map.
    #[test]
    fn test_set_state_attaches_en_passant_flag() {
        let mut game = load_game();
        insert(&mut game, pos![3, 1], "DOUBLE_PUSHER", "WHITE");

        pick(&mut game, pos![3, 1]).unwrap();
        assert!(
            targets(&game).contains(&pos![3u8, 3u8]),
            "Double push to [3,3] should be available"
        );

        go(&mut game, pos![3, 3]).unwrap();

        let piece = game
            .state
            .pieces
            .get(&pos![3u8, 3u8])
            .expect("DOUBLE_PUSHER should be at [3,3] after the move");
        assert!(
            piece.state.contains_key("EN_PASSANT"),
            "EN_PASSANT flag should be set on the piece after a double push"
        );
    }

    // -------------------------------------------------------------------------
    // REMOVE beside the target (en passant)
    // -------------------------------------------------------------------------

    /// WHITE EN_PASSANT_CAPTOR at [3,5], BLACK DOUBLE_PUSHER at [4,5] with EN_PASSANT flag.
    /// The captor's move (step [1,1]) onto an empty square is gated by a COUNT of EN_PASSANT at
    /// [4,5], and removes the piece at [4,5].
    /// After executing the move to [4,6]:
    ///   - WHITE piece lands at [4,6]
    ///   - [3,5] (source) is cleared
    ///   - [4,5] (en-passant pawn) is cleared
    #[test]
    fn test_en_passant_capture_removes_captured_pawn() {
        let mut game = load_game();
        insert(&mut game, pos![3, 5], "EN_PASSANT_CAPTOR", "WHITE");
        insert(&mut game, pos![4, 5], "DOUBLE_PUSHER", "BLACK");

        // Manually set EN_PASSANT flag (simulates the flag set last turn and ticked once).
        game.state
            .pieces
            .get_mut(&pos![4u8, 5u8])
            .unwrap()
            .state
            .insert("EN_PASSANT".to_string(), PieceState::Uint(0));

        pick(&mut game, pos![3, 5]).unwrap();
        assert!(
            targets(&game).contains(&pos![4u8, 6u8]),
            "En passant move to [4,6] should be available when neighbour has EN_PASSANT flag"
        );

        go(&mut game, pos![4, 6]).unwrap();

        assert!(
            game.state.pieces.get(&pos![4u8, 6u8]).is_some(),
            "WHITE piece should be at [4,6] after en passant"
        );
        assert_eq!(
            game.state.pieces.get(&pos![4u8, 6u8]).unwrap().player,
            "WHITE",
            "Piece at [4,6] should belong to WHITE"
        );
        assert!(
            game.state.pieces.get(&pos![4u8, 5u8]).is_none(),
            "BLACK pawn at [4,5] should be removed by the REMOVE effect"
        );
        assert!(
            game.state.pieces.get(&pos![3u8, 5u8]).is_none(),
            "Source square [3,5] should be cleared"
        );
    }

    /// Without the EN_PASSANT flag, the test of the move fails and the diagonal
    /// move to an empty square should NOT be available.
    #[test]
    fn test_en_passant_blocked_when_flag_absent() {
        let mut game = load_game();
        insert(&mut game, pos![3, 5], "EN_PASSANT_CAPTOR", "WHITE");
        insert(&mut game, pos![4, 5], "DOUBLE_PUSHER", "BLACK");
        // No EN_PASSANT flag set.

        pick(&mut game, pos![3, 5]).unwrap();
        let moves = targets(&game);
        // [4,6] should not be available (empty square, no EN_PASSANT flag on [4,5]).
        // [4,6] ENEMY capture would need an enemy piece there — it's empty, so ENEMY action won't fire.
        assert!(
            !moves.contains(&pos![4u8, 6u8]),
            "[4,6] should not be available: EN_PASSANT flag is absent"
        );
    }

    // -------------------------------------------------------------------------
    // A second MOVE effect (castling-style rook repositioning)
    // -------------------------------------------------------------------------

    /// WHITE CASTLER at [0,0], WHITE ROOK_PARTNER at [3,0].
    /// CASTLER moves 2 squares right to [2,0] (FIRST_MOVE + PATH_EMPTY satisfied).
    /// The second MOVE effect repositions the ROOK_PARTNER from [3,0] to [1,0].
    #[test]
    fn test_castling_side_effect_moves_rook_partner() {
        let mut game = load_game();
        insert(&mut game, pos![0, 0], "CASTLER", "WHITE");
        insert(&mut game, pos![3, 0], "ROOK_PARTNER", "WHITE");

        pick(&mut game, pos![0, 0]).unwrap();
        assert!(
            targets(&game).contains(&pos![2u8, 0u8]),
            "Castle to [2,0] should be available"
        );

        go(&mut game, pos![2, 0]).unwrap();

        assert!(
            game.state.pieces.get(&pos![2u8, 0u8]).is_some(),
            "CASTLER should be at [2,0]"
        );
        assert!(
            game.state.pieces.get(&pos![0u8, 0u8]).is_none(),
            "Source [0,0] should be cleared"
        );
        assert!(
            game.state.pieces.get(&pos![1u8, 0u8]).is_some(),
            "ROOK_PARTNER should have moved to [1,0]"
        );
        assert_eq!(
            game.state.pieces.get(&pos![1u8, 0u8]).unwrap().code,
            "ROOK_PARTNER",
            "Piece at [1,0] should be the ROOK_PARTNER"
        );
        assert!(
            game.state.pieces.get(&pos![3u8, 0u8]).is_none(),
            "ROOK_PARTNER's original square [3,0] should be cleared"
        );
    }

    /// The second MOVE effect is a no-op when the referenced partner piece is absent.
    /// The move itself must still be legal and should still move CASTLER to [2,0].
    #[test]
    fn test_castling_side_effect_noops_when_partner_missing() {
        let mut game = load_game();
        insert(&mut game, pos![0, 0], "CASTLER", "WHITE");

        pick(&mut game, pos![0, 0]).unwrap();
        assert!(
            targets(&game).contains(&pos![2u8, 0u8]),
            "Castle to [2,0] should still be available without a rook partner"
        );

        go(&mut game, pos![2, 0]).unwrap();

        assert_eq!(
            game.state
                .pieces
                .get(&pos![2u8, 0u8])
                .map(|p| p.code.as_str()),
            Some("CASTLER"),
            "CASTLER should still land at [2,0]"
        );
        assert!(
            game.state.pieces.get(&pos![0u8, 0u8]).is_none(),
            "Source [0,0] should be cleared"
        );
        assert!(
            game.state.pieces.get(&pos![1u8, 0u8]).is_none(),
            "No rook should be materialized when the source of the second MOVE is absent"
        );
    }

    /// An effect of a kind the engine does not know is refused when the spec is read, not
    /// ignored: a move that did less than it says would be played wrongly.
    #[test]
    fn test_unknown_effect_is_refused() {
        let mut spec: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string("./src/tests/side_effects/spec.json").expect("the spec"),
        )
        .expect("the spec is JSON");
        let piece = spec["pieces"]
            .as_array_mut()
            .expect("pieces")
            .iter_mut()
            .find(|piece| piece["code"] == "UNKNOWN_SIDE_EFFECT_PIECE")
            .expect("the piece is in the spec");
        piece["moves"][0]["do"]
            .as_array_mut()
            .expect("steps")
            .push(serde_json::json!({ "effect": "UNKNOWN_ACTION" }));

        let error =
            parse_game_spec_str(&spec.to_string()).expect_err("an unknown effect is refused");
        assert!(
            error.to_string().contains("UNKNOWN_ACTION"),
            "the refusal names the effect: {error}"
        );
    }
}
