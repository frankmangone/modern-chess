#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load_game() -> Game {
        parse_game_spec("./specs/shogi.json")
            .map(Game::from_spec)
            .expect("Failed to load shogi spec")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, code: &str, player: &str) {
        game.state.pieces.insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    fn remove(game: &mut Game, pos: Vec<u8>) {
        game.state.pieces.remove(&pos);
    }

    // -----------------------------------------------------------------------
    // Test 1 — Spec loads without error
    // -----------------------------------------------------------------------
    #[test]
    fn test_shogi_spec_loads() {
        let game = load_game();
        assert_eq!(game.name, "SHOGI");
        assert!(game.hand_enabled);
        assert_eq!(game.leader, vec!["KING".to_string()]);
    }

    // -----------------------------------------------------------------------
    // Test 2 — Pawn moves forward only; does NOT move diagonally or backward
    //
    // SENTE PAWN at [4,4] on an empty board (no pieces nearby except self).
    // Should have exactly one available move: forward [4,5].
    // Should NOT have [3,5], [5,5], [4,3] (diagonal / backward).
    // -----------------------------------------------------------------------
    #[test]
    fn test_pawn_moves_forward_not_diagonally() {
        let mut game = load_game();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "PAWN", "SENTE");
        // Also place GOTE king so game-over detection doesn't crash
        insert(&mut game, vec![4, 8], "KING", "GOTE");
        insert(&mut game, vec![4, 0], "KING", "SENTE");

        game.transition(GameTransition::CalculateMoves { position: vec![4, 4] }).unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();

        assert!(moves.contains_key(&vec![4u8, 5u8]), "PAWN should move forward to [4,5]");
        assert!(!moves.contains_key(&vec![3u8, 5u8]), "PAWN must not move diagonally");
        assert!(!moves.contains_key(&vec![5u8, 5u8]), "PAWN must not move diagonally");
        assert!(!moves.contains_key(&vec![4u8, 3u8]), "PAWN must not move backward");
        assert_eq!(moves.len(), 1, "PAWN should have exactly 1 move");
    }

    // -----------------------------------------------------------------------
    // Test 3 — Pawn captures forward (not diagonally)
    //
    // SENTE PAWN at [4,4], GOTE PAWN at [4,5] (directly ahead).
    // Should be able to capture at [4,5] but not at [3,5] or [5,5].
    // -----------------------------------------------------------------------
    #[test]
    fn test_pawn_captures_forward() {
        let mut game = load_game();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "PAWN",  "SENTE");
        insert(&mut game, vec![4, 5], "PAWN",  "GOTE");
        insert(&mut game, vec![4, 8], "KING",  "GOTE");
        insert(&mut game, vec![4, 0], "KING",  "SENTE");

        game.transition(GameTransition::CalculateMoves { position: vec![4, 4] }).unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();

        assert!(moves.contains_key(&vec![4u8, 5u8]), "PAWN should capture the enemy pawn ahead");
        // Diagonal squares are NOT capture targets for Shogi pawn
        assert!(!moves.contains_key(&vec![3u8, 5u8]));
        assert!(!moves.contains_key(&vec![5u8, 5u8]));
    }

    // -----------------------------------------------------------------------
    // Test 4 — Knight jumps over pieces
    //
    // SENTE KNIGHT at [4,2]. Place a PAWN at [4,3] blocking the path.
    // Knight should still reach [5,4] and [3,4] (the L-shape destinations).
    // -----------------------------------------------------------------------
    #[test]
    fn test_knight_jumps_over_pieces() {
        let mut game = load_game();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 2], "KNIGHT", "SENTE");
        insert(&mut game, vec![4, 3], "PAWN",   "SENTE"); // blocker on path
        insert(&mut game, vec![4, 8], "KING",   "GOTE");
        insert(&mut game, vec![4, 0], "KING",   "SENTE");

        game.transition(GameTransition::CalculateMoves { position: vec![4, 2] }).unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();

        assert!(moves.contains_key(&vec![5u8, 4u8]), "Knight should jump to [5,4]");
        assert!(moves.contains_key(&vec![3u8, 4u8]), "Knight should jump to [3,4]");
    }

    // -----------------------------------------------------------------------
    // Test 5 — Lance slides forward only
    //
    // SENTE LANCE at [4,2] on an open board. Should see all forward squares
    // ([4,3]..[4,8]) and nothing else.
    // -----------------------------------------------------------------------
    #[test]
    fn test_lance_slides_forward_only() {
        let mut game = load_game();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 2], "LANCE", "SENTE");
        insert(&mut game, vec![4, 8], "KING",  "GOTE");
        insert(&mut game, vec![4, 0], "KING",  "SENTE");

        game.transition(GameTransition::CalculateMoves { position: vec![4, 2] }).unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();

        // Forward squares
        for row in 3u8..=7u8 {
            assert!(moves.contains_key(&vec![4u8, row]), "LANCE should reach [4,{}]", row);
        }
        // No sideways or backward
        assert!(!moves.contains_key(&vec![3u8, 2u8]));
        assert!(!moves.contains_key(&vec![5u8, 2u8]));
        assert!(!moves.contains_key(&vec![4u8, 1u8]));
        assert!(!moves.contains_key(&vec![4u8, 0u8]));
    }

    // -----------------------------------------------------------------------
    // Test 6 — Capturing a piece populates the capturer's hand
    //
    // SENTE GOLD at [4,4] captures GOTE SILVER at [4,5].
    // After the capture, SENTE's hand should contain 1 × SILVER.
    // -----------------------------------------------------------------------
    #[test]
    fn test_capture_populates_hand() {
        let mut game = load_game();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "GOLD",   "SENTE");
        insert(&mut game, vec![4, 5], "SILVER", "GOTE");
        insert(&mut game, vec![4, 8], "KING",   "GOTE");
        insert(&mut game, vec![4, 0], "KING",   "SENTE");

        game.transition(GameTransition::CalculateMoves { position: vec![4, 4] }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: vec![4, 5] }).unwrap();

        let sente_hand = game.state.hand.get("SENTE").expect("SENTE should have pieces in hand");
        assert_eq!(sente_hand.get("SILVER").copied().unwrap_or(0), 1,
            "SENTE should have 1 SILVER in hand after capturing it");
    }

    // -----------------------------------------------------------------------
    // Test 7 — Promoted piece enters hand as base form (demotes_to)
    //
    // SENTE GOLD captures GOTE DRAGON (promoted ROOK). DRAGON.demotes_to = ROOK,
    // so SENTE's hand should contain 1 × ROOK, not 1 × DRAGON.
    // -----------------------------------------------------------------------
    #[test]
    fn test_promoted_piece_demotes_in_hand() {
        let mut game = load_game();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "GOLD",   "SENTE");
        insert(&mut game, vec![4, 5], "DRAGON", "GOTE");
        insert(&mut game, vec![4, 8], "KING",   "GOTE");
        insert(&mut game, vec![4, 0], "KING",   "SENTE");

        game.transition(GameTransition::CalculateMoves { position: vec![4, 4] }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: vec![4, 5] }).unwrap();

        let sente_hand = game.state.hand.get("SENTE").expect("SENTE should have pieces in hand");
        assert_eq!(sente_hand.get("ROOK").copied().unwrap_or(0), 1,
            "Captured DRAGON should demote to ROOK in hand");
        assert_eq!(sente_hand.get("DRAGON").copied().unwrap_or(0), 0,
            "DRAGON should NOT appear in hand directly");
    }

    // -----------------------------------------------------------------------
    // Test 8 — Drop is offered on empty squares
    //
    // SENTE has 1 GOLD in hand (no drop restrictions on GOLD).
    // CalculateDrops should return multiple empty squares.
    // -----------------------------------------------------------------------
    #[test]
    fn test_drop_offered_on_empty_squares() {
        let mut game = load_game();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 8], "KING", "GOTE");
        insert(&mut game, vec![4, 0], "KING", "SENTE");

        // Give SENTE a GOLD in hand.
        game.state.hand.entry("SENTE".to_string()).or_default().insert("GOLD".to_string(), 1);

        game.transition(GameTransition::CalculateDrops { piece_code: "GOLD".to_string() }).unwrap();

        let drops = game.state.available_moves.as_ref().unwrap();
        // Board has 81 squares, 2 occupied by kings → 79 empty squares eligible.
        assert!(drops.len() >= 70, "GOLD should be droppable on most empty squares");
        assert!(matches!(game.state.phase, GamePhase::Dropping { .. }));
    }

    // -----------------------------------------------------------------------
    // Test 9 — Drop is blocked by PROMO_FORCED (LANCE cannot drop on last rank)
    //
    // SENTE LANCE drop: row 8 (SENTE's last rank) should be excluded.
    // -----------------------------------------------------------------------
    #[test]
    fn test_lance_cannot_be_dropped_on_last_rank() {
        let mut game = load_game();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 8], "KING", "GOTE");
        insert(&mut game, vec![4, 0], "KING", "SENTE");

        game.state.hand.entry("SENTE".to_string()).or_default().insert("LANCE".to_string(), 1);

        game.transition(GameTransition::CalculateDrops { piece_code: "LANCE".to_string() }).unwrap();
        let drops = game.state.available_moves.as_ref().unwrap();

        for col in 0u8..9u8 {
            assert!(!drops.contains_key(&vec![col, 8u8]),
                "LANCE drop blocked on last rank [col={},row=8]", col);
        }
    }

    // -----------------------------------------------------------------------
    // Test 10 — Nifu: PAWN cannot be dropped on a file that already has a PAWN
    //
    // SENTE PAWN at [4,3]. Trying to drop another PAWN on file 4 (any row)
    // must be blocked (nifu rule).
    // -----------------------------------------------------------------------
    #[test]
    fn test_nifu_pawn_drop_blocked_on_same_file() {
        let mut game = load_game();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 3], "PAWN", "SENTE"); // existing pawn on file 4
        insert(&mut game, vec![4, 8], "KING", "GOTE");
        insert(&mut game, vec![4, 0], "KING", "SENTE");

        game.state.hand.entry("SENTE".to_string()).or_default().insert("PAWN".to_string(), 1);

        game.transition(GameTransition::CalculateDrops { piece_code: "PAWN".to_string() }).unwrap();
        let drops = game.state.available_moves.as_ref().unwrap();

        // File 4 (col 4) should be completely blocked for pawn drops.
        for row in 0u8..8u8 {
            assert!(!drops.contains_key(&vec![4u8, row]),
                "Nifu: pawn drop on file 4 row {} must be blocked", row);
        }
        // But file 3 (col 3) should be available (no pawn there).
        let file3_available = (0u8..8u8).any(|row| drops.contains_key(&vec![3u8, row]));
        assert!(file3_available, "Pawn should be droppable on file 3");
    }

    // -----------------------------------------------------------------------
    // Test 11 — ExecuteDrop places the piece on the board and decrements hand
    // -----------------------------------------------------------------------
    #[test]
    fn test_execute_drop_places_piece_and_decrements_hand() {
        let mut game = load_game();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 8], "KING", "GOTE");
        insert(&mut game, vec![4, 0], "KING", "SENTE");

        game.state.hand.entry("SENTE".to_string()).or_default().insert("GOLD".to_string(), 1);

        game.transition(GameTransition::CalculateDrops { piece_code: "GOLD".to_string() }).unwrap();
        game.transition(GameTransition::ExecuteDrop { position: vec![4, 4] }).unwrap();

        // GOLD should now be on the board
        let piece = game.state.pieces.get(&vec![4u8, 4u8]).expect("piece should be at [4,4]");
        assert_eq!(piece.code, "GOLD");
        assert_eq!(piece.player, "SENTE");

        // Hand should be empty
        let hand_count = game.state.hand.get("SENTE")
            .and_then(|h| h.get("GOLD"))
            .copied()
            .unwrap_or(0);
        assert_eq!(hand_count, 0, "Hand should be decremented to 0");
    }

    // -----------------------------------------------------------------------
    // Test 12 — Promotion zone triggers Transforming phase
    //
    // SENTE PAWN at [4,7] (one step from row 8 = promotion zone).
    // Moving to [4,8] should trigger Transforming { options: ["TOKIN", "PAWN"] }
    // (optional promotion since PROMO_FORCED fires first for row 8 → ["TOKIN"]).
    // Actually row 8 IS PROMO_FORCED, so options will be ["TOKIN"] only.
    // -----------------------------------------------------------------------
    #[test]
    fn test_promotion_zone_triggers_transforming() {
        let mut game = load_game();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 7], "PAWN", "SENTE");
        insert(&mut game, vec![4, 8], "KING", "GOTE"); // remove this and use row 5 instead
        insert(&mut game, vec![4, 0], "KING", "SENTE");

        // Move GOTE king out of the way so PAWN can advance
        remove(&mut game, vec![4, 8]);
        insert(&mut game, vec![0, 8], "KING", "GOTE");

        game.transition(GameTransition::CalculateMoves { position: vec![4, 7] }).unwrap();
        assert!(game.state.available_moves.as_ref().unwrap().contains_key(&vec![4u8, 8u8]));

        game.transition(GameTransition::ExecuteMove { position: vec![4, 8] }).unwrap();

        match &game.state.phase {
            GamePhase::Transforming { position, options } => {
                assert_eq!(position, &vec![4u8, 8u8]);
                // Row 8 is PROMO_FORCED → forced promotion, only TOKIN
                assert_eq!(options, &vec!["TOKIN".to_string()]);
            }
            other => panic!("Expected Transforming phase, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Test 13 — Optional promotion in non-forced zone
    //
    // SENTE PAWN at [4,6] → moves to [4,7] (PROMO_ZONE but not PROMO_FORCED).
    // Options should be ["TOKIN", "PAWN"] (optional: can choose to promote or not).
    // -----------------------------------------------------------------------
    #[test]
    fn test_optional_promotion_in_promo_zone() {
        let mut game = load_game();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 5], "PAWN", "SENTE");
        insert(&mut game, vec![0, 8], "KING", "GOTE");
        insert(&mut game, vec![4, 0], "KING", "SENTE");

        game.transition(GameTransition::CalculateMoves { position: vec![4, 5] }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: vec![4, 6] }).unwrap();

        match &game.state.phase {
            GamePhase::Transforming { options, .. } => {
                assert!(options.contains(&"TOKIN".to_string()), "Should offer TOKIN");
                assert!(options.contains(&"PAWN".to_string()),  "Should offer PAWN (stay)");
            }
            other => panic!("Expected Transforming, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Test 14 — Game-over: drops count as legal moves (prevents false stalemate)
    //
    // Set up a position where the SENTE KING has no board moves but SENTE has
    // a piece in hand that can be dropped. any_legal_moves() must return true.
    // -----------------------------------------------------------------------
    #[test]
    fn test_drops_prevent_stalemate() {
        let mut game = load_game();
        game.state.pieces.clear();

        // Completely corner SENTE KING with GOTE pieces so it has no board moves,
        // then verify the game is still Idle because SENTE has a drop available.
        insert(&mut game, vec![0, 0], "KING",   "SENTE");
        // surround king: [1,0],[0,1],[1,1] are occupied by GOTE pieces
        insert(&mut game, vec![1, 0], "GOLD",   "GOTE"); // attacks [0,0], [2,0], [0,1], [1,1], [2,1]
        insert(&mut game, vec![0, 1], "GOLD",   "GOTE"); // covers [0,0],[1,1] area
        insert(&mut game, vec![8, 8], "KING",   "GOTE");

        // Give SENTE a GOLD in hand so they have a legal drop.
        game.state.hand.entry("SENTE".to_string()).or_default().insert("GOLD".to_string(), 1);

        assert!(game.any_legal_moves(),
            "any_legal_moves should return true when drops are available");
    }
}
