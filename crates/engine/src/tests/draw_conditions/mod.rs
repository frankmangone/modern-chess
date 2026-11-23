#[cfg(test)]
mod tests {
    use crate::logic::Piece;
    use crate::logic::{Game, GamePhase};
    use crate::specs::parse_game_spec;
    use crate::tests::support::{go, phase_is_choosing, pick, without_repetition};

    fn load_chess() -> Game {
        parse_game_spec("./specs/chess.json")
            .map(Game::from_spec)
            .expect("Failed to load chess spec")
    }

    /// Both players hold `value` quiet half-moves, as the game counts them for both.
    fn set_quiet(game: &mut Game, value: i64) {
        for player in ["WHITE", "BLACK"] {
            game.state
                .counters
                .entry(player.to_string())
                .or_default()
                .insert("QUIET".to_string(), value);
        }
    }

    fn quiet(game: &Game, player: &str) -> i64 {
        game.state
            .counters
            .get(player)
            .and_then(|counters| counters.get("QUIET"))
            .copied()
            .unwrap_or(0)
    }

    // -----------------------------------------------------------------------
    // Repetition draw
    // -----------------------------------------------------------------------

    #[test]
    fn chess_keeps_the_has_moved_flag_in_the_position_record() {
        let mut game = load_chess();
        assert!(game.rules.moved_matters, "castling reads `moved`");
        let start = game.position_key();
        // Both knights go out and come back: the board is the one it began with, but the
        // knights have moved, so the position is not.
        for (from, to) in [
            (pos![1, 0], pos![2, 2]),
            (pos![6, 7], pos![5, 5]),
            (pos![2, 2], pos![1, 0]),
            (pos![5, 5], pos![6, 7]),
        ] {
            pick(&mut game, from).unwrap();
            go(&mut game, to).unwrap();
        }
        assert_ne!(game.position_key(), start);
    }

    #[test]
    fn a_game_that_never_reads_moved_leaves_the_flag_out_of_the_record() {
        let game = parse_game_spec("./specs/pong_hau_ki.json")
            .map(Game::from_spec)
            .expect("Failed to load the Pong hau k'i spec");
        assert!(!game.rules.moved_matters);
    }

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

        // Each knight counts as moved once it has moved, so the position after 1.Nc3 Nf6
        // (both knights out, both moved) is the one seen a third time, on the tenth half-move.
        let moves = [
            (pos![1, 0], pos![2, 2]), // WHITE: b1 → c3
            (pos![6, 7], pos![5, 5]), // BLACK: g8 → f6
            (pos![2, 2], pos![1, 0]), // WHITE: c3 → b1
            (pos![5, 5], pos![6, 7]), // BLACK: f6 → g8
        ];
        let mut played = 0;
        for (from, to) in moves.iter().cycle().take(12) {
            if matches!(game.state.phase, GamePhase::GameOver { .. }) {
                break;
            }
            pick(&mut game, *from).unwrap();
            go(&mut game, *to).unwrap();
            played += 1;
        }
        assert_eq!(played, 10, "drawn when the position repeats a third time");

        assert!(
            matches!(game.state.phase, GamePhase::GameOver { winner: None }),
            "Should be draw by repetition after 3 occurrences, got {:?}",
            game.state.phase
        );
    }

    #[test]
    fn test_repetition_does_not_trigger_before_threshold() {
        let mut game = load_chess();

        // Two round-trips (position seen twice) — not yet three.
        for _ in 0..2 {
            pick(&mut game, pos![1, 0]).unwrap();
            go(&mut game, pos![2, 2]).unwrap();
            pick(&mut game, pos![6, 7]).unwrap();
            go(&mut game, pos![5, 5]).unwrap();
            pick(&mut game, pos![2, 2]).unwrap();
            go(&mut game, pos![1, 0]).unwrap();
            pick(&mut game, pos![5, 5]).unwrap();
            go(&mut game, pos![6, 7]).unwrap();
        }

        assert!(
            phase_is_choosing(&game),
            "Should still be choosing after only 2 repetitions"
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
        without_repetition(&mut game);

        // Remove all pawns from both sides so no pawn moves are available.
        for col in 0..8u8 {
            game.state.pieces.remove(&pos![col, 1]); // WHITE pawns
            game.state.pieces.remove(&pos![col, 6]); // BLACK pawns
        }
        // Also remove pieces that would block knight oscillation for both sides.
        // WHITE knight on b1=[1,0] ↔ c3=[2,2] (clear c3 area – it's empty in
        // standard setup).  BLACK knight on g8=[6,7] ↔ f6=[5,5].

        // 100 half-moves = 50 rounds of (WHITE bounce + BLACK bounce).
        for _ in 0..25 {
            // WHITE: b1 → c3
            pick(&mut game, pos![1, 0]).unwrap();
            go(&mut game, pos![2, 2]).unwrap();
            // BLACK: g8 → f6
            pick(&mut game, pos![6, 7]).unwrap();
            go(&mut game, pos![5, 5]).unwrap();
            // WHITE: c3 → b1
            pick(&mut game, pos![2, 2]).unwrap();
            go(&mut game, pos![1, 0]).unwrap();
            // BLACK: f6 → g8
            pick(&mut game, pos![5, 5]).unwrap();
            go(&mut game, pos![6, 7]).unwrap();
        }

        assert!(
            matches!(game.state.phase, GamePhase::GameOver { winner: None }),
            "Should be draw by fifty-move rule after 100 non-resetting half-moves, got {:?}",
            game.state.phase
        );
    }

    #[test]
    fn test_fifty_move_rule_counts_quiet_half_moves_for_both_players() {
        let mut game = load_chess();
        pick(&mut game, pos![1, 0]).unwrap();
        go(&mut game, pos![2, 2]).unwrap();
        pick(&mut game, pos![6, 7]).unwrap();
        go(&mut game, pos![5, 5]).unwrap();

        assert_eq!(quiet(&game, "WHITE"), 2);
        assert_eq!(quiet(&game, "BLACK"), 2);
    }

    #[test]
    fn test_fifty_move_rule_triggers_at_the_hundredth_quiet_half_move() {
        let mut game = load_chess();
        without_repetition(&mut game);
        set_quiet(&mut game, 99);

        pick(&mut game, pos![1, 0]).unwrap();
        go(&mut game, pos![2, 2]).unwrap();

        assert!(
            matches!(game.state.phase, GamePhase::GameOver { winner: None }),
            "the hundredth quiet half-move is a draw, got {:?}",
            game.state.phase
        );
    }

    #[test]
    fn test_fifty_move_rule_resets_on_capture() {
        let mut game = load_chess();
        without_repetition(&mut game);
        set_quiet(&mut game, 99);

        // A BLACK pawn on c3 for WHITE's knight on b1 to take.
        game.state.pieces.insert(
            pos![2, 2],
            Piece::new("PAWN".to_string(), "BLACK".to_string()),
        );
        pick(&mut game, pos![1, 0]).unwrap();
        go(&mut game, pos![2, 2]).unwrap();

        assert!(
            !matches!(game.state.phase, GamePhase::GameOver { .. }),
            "A capture resets the count; no draw expected, got {:?}",
            game.state.phase
        );
        assert_eq!(quiet(&game, "WHITE"), 0);
        assert_eq!(quiet(&game, "BLACK"), 0);
    }

    #[test]
    fn test_fifty_move_rule_resets_on_pawn_move() {
        let mut game = load_chess();
        without_repetition(&mut game);
        set_quiet(&mut game, 99);

        // WHITE pawn push — resets counter.
        pick(&mut game, pos![4, 1]).unwrap();
        go(&mut game, pos![4, 3]).unwrap();
        assert!(
            !matches!(game.state.phase, GamePhase::GameOver { .. }),
            "Pawn push should have reset the fifty-move counter; no draw expected"
        );
        assert_eq!(quiet(&game, "BLACK"), 0);

        // A quiet move afterwards starts the count again.
        pick(&mut game, pos![6, 7]).unwrap();
        go(&mut game, pos![5, 5]).unwrap();
        assert_eq!(quiet(&game, "WHITE"), 1);
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
        pick(&mut game, pos![4, 0]).unwrap();
        go(&mut game, pos![4, 1]).unwrap();

        assert!(
            matches!(game.state.phase, GamePhase::GameOver { winner: None }),
            "King vs King should be a draw, got {:?}",
            game.state.phase
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
        let white_bishop_pos = game
            .state
            .pieces
            .iter()
            .find(|(_, p)| p.player == "WHITE" && p.code == "BISHOP")
            .map(|(pos, _)| *pos);
        if let Some(pos) = white_bishop_pos {
            let all_white_bishops: Vec<_> = game
                .state
                .pieces
                .keys()
                .filter(|p| {
                    game.state.pieces[*p].player == "WHITE"
                        && game.state.pieces[*p].code == "BISHOP"
                })
                .cloned()
                .collect();
            for p in all_white_bishops.into_iter().skip(1) {
                game.state.pieces.remove(&p);
            }
            let _ = pos; // suppress unused warning
        }

        // Move WHITE king to trigger check.
        game.state.pieces.remove(&pos![4u8, 1u8]); // clear pawn in front
        pick(&mut game, pos![4, 0]).unwrap();
        go(&mut game, pos![4, 1]).unwrap();

        assert!(
            matches!(game.state.phase, GamePhase::GameOver { winner: None }),
            "King+Bishop vs King should be a draw, got {:?}",
            game.state.phase
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
        let extra: Vec<_> = game
            .state
            .pieces
            .iter()
            .filter(|(_, p)| p.player == "WHITE" && p.code == "ROOK")
            .skip(1)
            .map(|(pos, _)| *pos)
            .collect();
        for p in extra {
            game.state.pieces.remove(&p);
        }

        game.state.pieces.remove(&pos![4u8, 1u8]); // clear pawn
        pick(&mut game, pos![4, 0]).unwrap();
        go(&mut game, pos![4, 1]).unwrap();

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
        pick(&mut game, pos![1, 0]).unwrap();
        go(&mut game, pos![2, 2]).unwrap();
        pick(&mut game, pos![6, 7]).unwrap();
        go(&mut game, pos![5, 5]).unwrap();
        pick(&mut game, pos![2, 2]).unwrap();
        go(&mut game, pos![1, 0]).unwrap();
        pick(&mut game, pos![5, 5]).unwrap();
        go(&mut game, pos![6, 7]).unwrap();

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
