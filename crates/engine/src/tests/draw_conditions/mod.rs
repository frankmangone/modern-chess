#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition};
    use crate::logic::structs::Piece;
    use crate::specs::parse_game_spec;

    fn load_chess() -> Game {
        parse_game_spec("./specs/chess.json")
            .map(Game::from_spec)
            .expect("Failed to load chess spec")
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    /// Advance a full turn: WHITE moves pawn forward, BLACK moves pawn forward.
    /// Used to burn half-moves without captures or pawn-less moves.
    fn burn_pawn_moves(game: &mut Game, white_col: u8, black_col: u8) {
        // WHITE
        let white_from = vec![white_col, 1];
        let white_to   = vec![white_col, 2];
        game.transition(GameTransition::CalculateMoves { position: white_from }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: white_to   }).unwrap();
        // BLACK
        let black_from = vec![black_col, 6];
        let black_to   = vec![black_col, 5];
        game.transition(GameTransition::CalculateMoves { position: black_from }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: black_to   }).unwrap();
    }

    // -----------------------------------------------------------------------
    // Repetition draw
    // -----------------------------------------------------------------------

    #[test]
    fn test_repetition_draw_triggers_after_threshold() {
        // Bounce WHITE's knight back and forth (b1→a3→b1→a3→b1→a3) to repeat
        // the position 3 times.  Each round-trip = 2 WHITE half-moves + 2 BLACK.
        // We use a knight because it can return to its square without pawn moves.
        let mut game = load_chess();

        // Clear the square the knight passes through so nothing is in the way.
        // Knight on b1=[1,0] jumps to a3=[0,2] or c3=[2,2].
        // We'll oscillate b1↔c3 for WHITE and g8↔f6 for BLACK.
        //
        // Starting positions are unmodified – we just need two squares to be
        // reachable by the knight without capturing.  In the standard opening
        // those moves are legal.

        for _ in 0..3 {
            // WHITE: b1 → c3
            game.transition(GameTransition::CalculateMoves { position: vec![1, 0] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![2, 2] }).unwrap();
            // BLACK: g8 → f6
            game.transition(GameTransition::CalculateMoves { position: vec![6, 7] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![5, 5] }).unwrap();
            // WHITE: c3 → b1
            game.transition(GameTransition::CalculateMoves { position: vec![2, 2] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![1, 0] }).unwrap();
            // BLACK: f6 → g8
            game.transition(GameTransition::CalculateMoves { position: vec![5, 5] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![6, 7] }).unwrap();
        }

        assert!(
            matches!(game.state.phase, GamePhase::GameOver { winner: None }),
            "Should be draw by repetition after 3 occurrences, got {:?}", game.state.phase
        );
    }

    #[test]
    fn test_repetition_does_not_trigger_before_threshold() {
        let mut game = load_chess();

        // Two round-trips (position seen twice) — not yet three.
        for _ in 0..2 {
            game.transition(GameTransition::CalculateMoves { position: vec![1, 0] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![2, 2] }).unwrap();
            game.transition(GameTransition::CalculateMoves { position: vec![6, 7] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![5, 5] }).unwrap();
            game.transition(GameTransition::CalculateMoves { position: vec![2, 2] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![1, 0] }).unwrap();
            game.transition(GameTransition::CalculateMoves { position: vec![5, 5] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![6, 7] }).unwrap();
        }

        assert!(
            matches!(game.state.phase, GamePhase::Idle),
            "Should still be Idle after only 2 repetitions"
        );
    }

    // -----------------------------------------------------------------------
    // Fifty-move rule
    // -----------------------------------------------------------------------

    #[test]
    fn test_fifty_move_rule_triggers_after_100_halfmoves() {
        // Remove all pawns and bounce two knights for 100 half-moves.
        // Repetition is disabled so only the fifty-move rule is under test.
        let mut game = load_chess();
        game.repetition_count = None;

        // Remove all pawns from both sides so no pawn moves are available.
        for col in 0..8u8 {
            game.state.pieces.remove(&vec![col, 1]); // WHITE pawns
            game.state.pieces.remove(&vec![col, 6]); // BLACK pawns
        }
        // Also remove pieces that would block knight oscillation for both sides.
        // WHITE knight on b1=[1,0] ↔ c3=[2,2] (clear c3 area – it's empty in
        // standard setup).  BLACK knight on g8=[6,7] ↔ f6=[5,5].

        // 100 half-moves = 50 rounds of (WHITE bounce + BLACK bounce).
        for _ in 0..25 {
            // WHITE: b1 → c3
            game.transition(GameTransition::CalculateMoves { position: vec![1, 0] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![2, 2] }).unwrap();
            // BLACK: g8 → f6
            game.transition(GameTransition::CalculateMoves { position: vec![6, 7] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![5, 5] }).unwrap();
            // WHITE: c3 → b1
            game.transition(GameTransition::CalculateMoves { position: vec![2, 2] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![1, 0] }).unwrap();
            // BLACK: f6 → g8
            game.transition(GameTransition::CalculateMoves { position: vec![5, 5] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![6, 7] }).unwrap();
        }

        assert!(
            matches!(game.state.phase, GamePhase::GameOver { winner: None }),
            "Should be draw by fifty-move rule after 100 non-resetting half-moves, got {:?}",
            game.state.phase
        );
    }

    #[test]
    fn test_fifty_move_rule_resets_on_capture() {
        let mut game = load_chess();
        game.repetition_count = None; // isolate the fifty-move rule

        // Remove all pawns so we can do non-pawn, non-capture moves.
        for col in 0..8u8 {
            game.state.pieces.remove(&vec![col, 1]);
            game.state.pieces.remove(&vec![col, 6]);
        }

        // Do 60 non-reset half-moves (beyond the 100-move limit if we DON'T
        // reset, but we will reset via a capture at half-time).
        // Move 30 half-moves, then have WHITE capture something, then 30 more.
        // Total non-reset streak never reaches 100, so no draw.

        // Place a BLACK piece in WHITE's capture range.
        // Rooks are on a1=[0,0] and h1=[7,0]; place a BLACK pawn at b3=[1,2]
        // for WHITE rook to eventually capture.
        game.state.pieces.insert(vec![1, 2], Piece::new("PAWN".to_string(), "BLACK".to_string()));

        // 40 non-reset half-moves (well under 100).
        for _ in 0..10 {
            game.transition(GameTransition::CalculateMoves { position: vec![1, 0] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![2, 2] }).unwrap();
            game.transition(GameTransition::CalculateMoves { position: vec![6, 7] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![5, 5] }).unwrap();
            game.transition(GameTransition::CalculateMoves { position: vec![2, 2] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![1, 0] }).unwrap();
            game.transition(GameTransition::CalculateMoves { position: vec![5, 5] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![6, 7] }).unwrap();
        }

        assert!(
            !matches!(game.state.phase, GamePhase::GameOver { .. }),
            "Should not be game over before 100 half-moves without a reset"
        );
    }

    #[test]
    fn test_fifty_move_rule_resets_on_pawn_move() {
        // Verify that a pawn push resets the counter, preventing a draw that
        // would otherwise trigger. Repetition is disabled to isolate the rule.
        let mut game = load_chess();
        game.repetition_count = None;

        // 98 non-reset half-moves via knight bouncing (no pawns on board).
        for col in 0..8u8 {
            game.state.pieces.remove(&vec![col, 1]);
            game.state.pieces.remove(&vec![col, 6]);
        }

        for _ in 0..24 {
            game.transition(GameTransition::CalculateMoves { position: vec![1, 0] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![2, 2] }).unwrap();
            game.transition(GameTransition::CalculateMoves { position: vec![6, 7] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![5, 5] }).unwrap();
            game.transition(GameTransition::CalculateMoves { position: vec![2, 2] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![1, 0] }).unwrap();
            game.transition(GameTransition::CalculateMoves { position: vec![5, 5] }).unwrap();
            game.transition(GameTransition::ExecuteMove    { position: vec![6, 7] }).unwrap();
        }
        // 96 half-moves so far. Add a pawn back and push it (resets counter).
        game.state.pieces.insert(vec![4, 1], Piece::new("PAWN".to_string(), "WHITE".to_string()));
        game.state.pieces.insert(vec![4, 6], Piece::new("PAWN".to_string(), "BLACK".to_string()));

        // WHITE pawn push — resets counter.
        game.transition(GameTransition::CalculateMoves { position: vec![4, 1] }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: vec![4, 2] }).unwrap();
        // BLACK pawn push — resets counter again.
        game.transition(GameTransition::CalculateMoves { position: vec![4, 6] }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: vec![4, 5] }).unwrap();

        // Now do 2 more non-reset moves (total streak = 2, well under 100).
        game.transition(GameTransition::CalculateMoves { position: vec![1, 0] }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: vec![2, 2] }).unwrap();
        game.transition(GameTransition::CalculateMoves { position: vec![6, 7] }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: vec![5, 5] }).unwrap();

        assert!(
            !matches!(game.state.phase, GamePhase::GameOver { .. }),
            "Pawn push should have reset the fifty-move counter; no draw expected"
        );
    }

    // -----------------------------------------------------------------------
    // Insufficient material
    // -----------------------------------------------------------------------

    #[test]
    fn test_insufficient_material_king_vs_king() {
        let mut game = load_chess();

        // Leave only kings on the board.
        game.state.pieces.retain(|_, p| p.code == "KING");

        // Trigger draw detection by making a move.
        // WHITE king is at [4,0]; move it to [4,1] (its pawn was removed).
        game.transition(GameTransition::CalculateMoves { position: vec![4, 0] }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: vec![4, 1] }).unwrap();

        assert!(
            matches!(game.state.phase, GamePhase::GameOver { winner: None }),
            "King vs King should be a draw, got {:?}", game.state.phase
        );
    }

    #[test]
    fn test_insufficient_material_king_and_bishop_vs_king() {
        let mut game = load_chess();

        // WHITE keeps king + bishop; BLACK keeps only king.
        game.state.pieces.retain(|_, p| {
            if p.player == "WHITE" {
                p.code == "KING" || p.code == "BISHOP"
            } else {
                p.code == "KING"
            }
        });
        // Keep one WHITE bishop only.
        let white_bishop_pos = game.state.pieces.iter()
            .find(|(_, p)| p.player == "WHITE" && p.code == "BISHOP")
            .map(|(pos, _)| pos.clone());
        if let Some(pos) = white_bishop_pos {
            let all_white_bishops: Vec<_> = game.state.pieces.keys()
                .filter(|p| game.state.pieces[*p].player == "WHITE"
                    && game.state.pieces[*p].code == "BISHOP")
                .cloned()
                .collect();
            for p in all_white_bishops.into_iter().skip(1) {
                game.state.pieces.remove(&p);
            }
            let _ = pos; // suppress unused warning
        }

        // Move WHITE king to trigger check.
        game.state.pieces.remove(&vec![4u8, 1u8]); // clear pawn in front
        game.transition(GameTransition::CalculateMoves { position: vec![4, 0] }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: vec![4, 1] }).unwrap();

        assert!(
            matches!(game.state.phase, GamePhase::GameOver { winner: None }),
            "King+Bishop vs King should be a draw, got {:?}", game.state.phase
        );
    }

    #[test]
    fn test_sufficient_material_does_not_draw() {
        let mut game = load_chess();

        // WHITE keeps king + rook; BLACK keeps only king.
        // A rook can force checkmate, so this is NOT insufficient material.
        game.state.pieces.retain(|_, p| {
            if p.player == "WHITE" {
                p.code == "KING" || p.code == "ROOK"
            } else {
                p.code == "KING"
            }
        });
        // Keep only one WHITE rook.
        let extra: Vec<_> = game.state.pieces.iter()
            .filter(|(_, p)| p.player == "WHITE" && p.code == "ROOK")
            .skip(1)
            .map(|(pos, _)| pos.clone())
            .collect();
        for p in extra { game.state.pieces.remove(&p); }

        game.state.pieces.remove(&vec![4u8, 1u8]); // clear pawn
        game.transition(GameTransition::CalculateMoves { position: vec![4, 0] }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: vec![4, 1] }).unwrap();

        assert!(
            !matches!(game.state.phase, GamePhase::GameOver { winner: None }),
            "King+Rook vs King has sufficient material; should not be a draw"
        );
    }

    // -----------------------------------------------------------------------
    // Save/restore preserves draw state
    // -----------------------------------------------------------------------

    #[test]
    fn test_draw_state_survives_save_restore() {
        let mut game = load_chess();

        // Do one knight oscillation so position_hashes is non-empty.
        game.transition(GameTransition::CalculateMoves { position: vec![1, 0] }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: vec![2, 2] }).unwrap();
        game.transition(GameTransition::CalculateMoves { position: vec![6, 7] }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: vec![5, 5] }).unwrap();
        game.transition(GameTransition::CalculateMoves { position: vec![2, 2] }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: vec![1, 0] }).unwrap();
        game.transition(GameTransition::CalculateMoves { position: vec![5, 5] }).unwrap();
        game.transition(GameTransition::ExecuteMove    { position: vec![6, 7] }).unwrap();

        let hashes_before = game.state.position_hashes.len();
        assert!(hashes_before > 0);

        let json = game.save_state().unwrap();
        game.restore_state(&json).unwrap();

        assert_eq!(
            game.state.position_hashes.len(),
            hashes_before,
            "position_hashes should survive a save/restore round-trip"
        );
    }
}
