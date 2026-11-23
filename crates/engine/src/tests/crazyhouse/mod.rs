#[cfg(test)]
mod tests {
    use crate::logic::{Game, Piece};
    use crate::shared::Position;
    use crate::specs::parse_game_spec;
    use crate::tests::support::{drop, drop_squares, go, pick, promotion_options};

    fn load_game() -> Game {
        parse_game_spec("./specs/crazyhouse.json")
            .map(Game::from_spec)
            .expect("Failed to load crazyhouse spec")
    }

    fn insert(game: &mut Game, pos: Position, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    // -----------------------------------------------------------------------
    // Test 1 — Spec loads; hand_enabled is true
    // -----------------------------------------------------------------------
    #[test]
    fn test_crazyhouse_spec_loads() {
        let game = load_game();
        assert_eq!(game.rules.name, "CRAZYHOUSE");
        assert!(game.rules.hand_enabled);
        assert_eq!(game.rules.leader, vec!["KING".to_string()]);
    }

    // -----------------------------------------------------------------------
    // Test 2 — Capturing a native piece adds it to the capturer's hand as-is.
    //
    // WHITE ROOK at [4,4] captures BLACK KNIGHT at [4,5].
    // BLACK's hand should be empty. WHITE's hand should be empty.
    // BLACK's hand after BLACK captures: not tested here.
    // WHITE captures BLACK KNIGHT → WHITE's hand gains KNIGHT (no demotes_to).
    // -----------------------------------------------------------------------
    #[test]
    fn test_capturing_native_piece_goes_to_hand_unchanged() {
        let mut game = load_game();
        game.state.pieces.clear();
        insert(&mut game, pos![4, 4], "ROOK", "WHITE");
        insert(&mut game, pos![4, 5], "KNIGHT", "BLACK");
        insert(&mut game, pos![4, 7], "KING", "BLACK");
        insert(&mut game, pos![4, 0], "KING", "WHITE");

        pick(&mut game, pos![4, 4]).unwrap();
        go(&mut game, pos![4, 5]).unwrap();

        let hand = game.hand();
        let white_hand = hand.get("WHITE").expect("WHITE should have a hand entry");
        assert_eq!(
            white_hand.get("KNIGHT"),
            Some(&1),
            "WHITE should have 1 KNIGHT in hand"
        );
        // The piece entered the hand as KNIGHT, not as something demoted
        assert!(
            white_hand.get("PAWN").is_none(),
            "no PAWN should be in hand"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3 — Capturing a promoted-pawn piece (P_QUEEN) adds PAWN to hand.
    //
    // WHITE ROOK at [4,4] captures BLACK P_QUEEN at [4,5].
    // P_QUEEN has demotes_to: "PAWN", so WHITE's hand should gain a PAWN.
    // -----------------------------------------------------------------------
    #[test]
    fn test_capturing_promoted_pawn_demotes_to_pawn_in_hand() {
        let mut game = load_game();
        game.state.pieces.clear();
        insert(&mut game, pos![4, 4], "ROOK", "WHITE");
        insert(&mut game, pos![4, 5], "P_QUEEN", "BLACK");
        insert(&mut game, pos![4, 7], "KING", "BLACK");
        insert(&mut game, pos![4, 0], "KING", "WHITE");

        pick(&mut game, pos![4, 4]).unwrap();
        go(&mut game, pos![4, 5]).unwrap();

        let hand = game.hand();
        let white_hand = hand.get("WHITE").expect("WHITE should have a hand entry");
        assert_eq!(
            white_hand.get("PAWN"),
            Some(&1),
            "captured P_QUEEN should demote to PAWN in hand"
        );
        assert!(
            white_hand.get("P_QUEEN").is_none(),
            "P_QUEEN should not be in hand directly"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4 — Pawn promotes to P_QUEEN (not QUEEN).
    //
    // WHITE PAWN at [3,6] advances to [3,7] — the back rank triggers promotion.
    // The engine should ask for the promotion, with options
    // ["P_QUEEN", "P_ROOK", "P_BISHOP", "P_KNIGHT"].
    // -----------------------------------------------------------------------
    #[test]
    fn test_pawn_promotes_to_p_queen_not_queen() {
        let mut game = load_game();
        game.state.pieces.clear();
        insert(&mut game, pos![3, 6], "PAWN", "WHITE");
        insert(&mut game, pos![4, 7], "KING", "BLACK");
        insert(&mut game, pos![4, 0], "KING", "WHITE");

        pick(&mut game, pos![3, 6]).unwrap();
        go(&mut game, pos![3, 7]).unwrap();

        // Should be asking for the promotion
        let options = promotion_options(&game);
        assert!(
            !options.is_empty(),
            "Expected a promotion to be asked, got {:?}",
            game.state.phase
        );
        assert!(
            options.contains(&"P_QUEEN".to_string()),
            "P_QUEEN should be an option"
        );
        assert!(
            options.contains(&"P_ROOK".to_string()),
            "P_ROOK should be an option"
        );
        assert!(
            options.contains(&"P_BISHOP".to_string()),
            "P_BISHOP should be an option"
        );
        assert!(
            options.contains(&"P_KNIGHT".to_string()),
            "P_KNIGHT should be an option"
        );
        assert!(
            !options.contains(&"QUEEN".to_string()),
            "QUEEN must NOT be offered"
        );
        assert!(
            !options.contains(&"ROOK".to_string()),
            "ROOK must NOT be offered"
        );
    }

    // -----------------------------------------------------------------------
    // Test 5 — Dropped pawn cannot land on terminal ranks (rows 0 and 7).
    //
    // WHITE has a PAWN in hand. Row 7 (WHITE's promotion rank) and row 0
    // (BLACK's promotion rank) should both be unavailable drop squares.
    // Squares on rows 1–6 should be available (if empty).
    // -----------------------------------------------------------------------
    #[test]
    fn test_pawn_cannot_drop_on_terminal_ranks() {
        let mut game = load_game();
        game.state.pieces.clear();
        insert(&mut game, pos![4, 7], "KING", "BLACK");
        insert(&mut game, pos![4, 0], "KING", "WHITE");

        // Give WHITE a pawn in hand
        game.state
            .hand
            .entry("WHITE".to_string())
            .or_default()
            .insert("PAWN".to_string(), 1);

        let drops = drop_squares(&game, "PAWN");

        // No square on row 0 or row 7 should be available
        for col in 0u8..8 {
            assert!(
                !drops.contains(&pos![col, 0]),
                "PAWN drop on [{col},0] (row 0) must be blocked"
            );
            assert!(
                !drops.contains(&pos![col, 7]),
                "PAWN drop on [{col},7] (row 7) must be blocked"
            );
        }

        // Some square on an interior row should be available
        assert!(
            !drops.is_empty(),
            "there should be legal drop squares on interior rows"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6 — Non-pawn pieces (QUEEN, ROOK, BISHOP, KNIGHT) have no drop
    //          restrictions: they can be dropped on any empty square.
    //
    // WHITE has a QUEEN in hand. All empty squares including row 0 and row 7
    // (except the squares occupied by kings) should be available.
    // -----------------------------------------------------------------------
    #[test]
    fn test_queen_can_drop_on_any_empty_square() {
        let mut game = load_game();
        game.state.pieces.clear();
        insert(&mut game, pos![4, 7], "KING", "BLACK");
        insert(&mut game, pos![4, 0], "KING", "WHITE");

        game.state
            .hand
            .entry("WHITE".to_string())
            .or_default()
            .insert("QUEEN".to_string(), 1);

        let drops = drop_squares(&game, "QUEEN");

        // Row 7 (except [4,7] which has BLACK KING) should have 7 drop squares
        let row7_drops: Vec<_> = (0u8..8)
            .filter(|&col| drops.contains(&pos![col, 7]))
            .collect();
        assert_eq!(
            row7_drops.len(),
            7,
            "QUEEN should be droppable on 7 of 8 squares on row 7 (one occupied by king)"
        );

        // Row 0 (except [4,0] which has WHITE KING) should have 7 drop squares
        let row0_drops: Vec<_> = (0u8..8)
            .filter(|&col| drops.contains(&pos![col, 0]))
            .collect();
        assert_eq!(
            row0_drops.len(),
            7,
            "QUEEN should be droppable on 7 of 8 squares on row 0 (one occupied by king)"
        );
    }

    // -----------------------------------------------------------------------
    // Test 7 — Full capture-then-drop cycle.
    //
    // WHITE captures BLACK BISHOP → WHITE hand gains BISHOP.
    // (Turn advances to BLACK.) BLACK makes a harmless move.
    // Then WHITE drops the BISHOP from hand.
    // Board should have WHITE BISHOP at the drop square; hand count decrements.
    // -----------------------------------------------------------------------
    #[test]
    fn test_capture_then_drop_full_cycle() {
        let mut game = load_game();
        game.state.pieces.clear();
        insert(&mut game, pos![4, 4], "ROOK", "WHITE");
        insert(&mut game, pos![4, 5], "BISHOP", "BLACK");
        insert(&mut game, pos![0, 7], "KING", "BLACK");
        insert(&mut game, pos![0, 0], "KING", "WHITE");

        // WHITE captures the BLACK BISHOP
        pick(&mut game, pos![4, 4]).unwrap();
        go(&mut game, pos![4, 5]).unwrap();

        // WHITE hand should have 1 BISHOP
        assert_eq!(
            game.hand().get("WHITE").and_then(|h| h.get("BISHOP")),
            Some(&1)
        );

        // BLACK makes a harmless king move
        pick(&mut game, pos![0, 7]).unwrap();
        go(&mut game, pos![1, 7]).unwrap();

        // WHITE drops the BISHOP on [2, 3]
        drop(&mut game, "BISHOP", pos![2, 3]).unwrap();

        // Board should have WHITE BISHOP at [2,3]
        let piece = game
            .piece_at_position(&pos![2, 3])
            .expect("should have a piece at [2,3]");
        assert_eq!(piece.code, "BISHOP");
        assert_eq!(piece.player, "WHITE");

        // Hand count should now be 0
        let count = game
            .hand()
            .get("WHITE")
            .and_then(|h| h.get("BISHOP"))
            .copied()
            .unwrap_or(0);
        assert_eq!(count, 0, "BISHOP count in hand should be 0 after dropping");
    }
}
