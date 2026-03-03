#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load_koth() -> Game {
        parse_game_spec("./specs/king_of_the_hill.json")
            .map(Game::from_spec)
            .expect("Failed to load king_of_the_hill spec")
    }

    fn load_shatranj() -> Game {
        parse_game_spec("./specs/shatranj.json")
            .map(Game::from_spec)
            .expect("Failed to load shatranj spec")
    }

    fn load_three_check() -> Game {
        parse_game_spec("./specs/three_check.json")
            .map(Game::from_spec)
            .expect("Failed to load three_check spec")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    // -----------------------------------------------------------------------
    // Test 1 — PIECE_IN_ZONE: king reaching the center wins immediately
    //
    // WHITE KING at [3,2]. BLACK KING at [7,7] (harmless, far from center).
    // WHITE moves KING one step north to [3,3], which is inside the CENTER zone
    // ([3,3],[4,3],[3,4],[4,4]).
    // Expected: GameOver { winner: Some("WHITE") }.
    // -----------------------------------------------------------------------

    #[test]
    fn test_piece_in_zone_king_of_the_hill() {
        let mut game = load_koth();
        game.state.pieces.clear();

        insert(&mut game, vec![3, 2], "KING", "WHITE");
        insert(&mut game, vec![7, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![3, 2],
        })
        .unwrap();
        assert!(
            game.state
                .available_moves
                .as_ref()
                .unwrap()
                .contains_key(&vec![3u8, 3u8]),
            "WHITE KING at [3,2] should be able to move to center [3,3]"
        );
        game.transition(GameTransition::ExecuteMove {
            position: vec![3, 3],
        })
        .unwrap();

        assert_eq!(
            game.state.phase,
            GamePhase::GameOver {
                winner: Some("WHITE".to_string())
            },
            "WHITE should win immediately upon reaching the center"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2 — PIECE_IN_ZONE: no win when king moves outside the center zone
    //
    // WHITE KING at [3,0] moves to [3,1] — not in the CENTER zone.
    // Game must continue.
    // -----------------------------------------------------------------------

    #[test]
    fn test_piece_in_zone_no_win_outside_center() {
        let mut game = load_koth();
        game.state.pieces.clear();

        insert(&mut game, vec![3, 0], "KING", "WHITE");
        insert(&mut game, vec![7, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![3, 0],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![3, 1],
        })
        .unwrap();

        assert_eq!(
            game.state.phase,
            GamePhase::Idle,
            "Game should continue when the king has not reached the center zone"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3 — OPPONENT_BARE: capturing the last non-exempt piece wins
    //
    // Shatranj: WHITE ALFIL at [2,2] captures BLACK FERZ at [4,4] (step [2,2]).
    // After the capture BLACK has only its SHAH, which is in the exempt list.
    // Expected: GameOver { winner: Some("WHITE") }.
    // -----------------------------------------------------------------------

    #[test]
    fn test_opponent_bare_wins_when_only_exempt_remains() {
        let mut game = load_shatranj();
        game.state.pieces.clear();

        insert(&mut game, vec![0, 0], "SHAH", "WHITE");
        insert(&mut game, vec![2, 2], "ALFIL", "WHITE");
        insert(&mut game, vec![7, 7], "SHAH", "BLACK");
        insert(&mut game, vec![4, 4], "FERZ", "BLACK");

        // WHITE ALFIL at [2,2] leaps to [4,4] capturing BLACK FERZ.
        game.transition(GameTransition::CalculateMoves {
            position: vec![2, 2],
        })
        .unwrap();
        assert!(
            game.state
                .available_moves
                .as_ref()
                .unwrap()
                .contains_key(&vec![4u8, 4u8]),
            "WHITE ALFIL should be able to capture BLACK FERZ at [4,4]"
        );
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 4],
        })
        .unwrap();

        assert_eq!(
            game.state.phase,
            GamePhase::GameOver {
                winner: Some("WHITE".to_string())
            },
            "WHITE should win when the opponent has only the exempt piece (SHAH) remaining"
        );
    }

    // -----------------------------------------------------------------------
    // Test 4 — OPPONENT_BARE: no win while non-exempt pieces remain
    //
    // WHITE FARAS at [2,1] captures BLACK FARAS at [4,2] (step [2,1]).
    // BLACK still has SHAH + FERZ → condition does NOT fire.
    // -----------------------------------------------------------------------

    #[test]
    fn test_opponent_bare_no_win_when_non_exempt_remains() {
        let mut game = load_shatranj();
        game.state.pieces.clear();

        insert(&mut game, vec![0, 0], "SHAH", "WHITE");
        insert(&mut game, vec![2, 1], "FARAS", "WHITE");
        insert(&mut game, vec![7, 7], "SHAH", "BLACK");
        insert(&mut game, vec![4, 2], "FARAS", "BLACK");
        insert(&mut game, vec![5, 5], "FERZ", "BLACK");

        // WHITE FARAS at [2,1] captures BLACK FARAS at [4,2] via step [2,1].
        game.transition(GameTransition::CalculateMoves {
            position: vec![2, 1],
        })
        .unwrap();
        assert!(
            game.state
                .available_moves
                .as_ref()
                .unwrap()
                .contains_key(&vec![4u8, 2u8]),
            "WHITE FARAS should be able to capture at [4,2]"
        );
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 2],
        })
        .unwrap();

        // BLACK still has SHAH + FERZ → not bare → game continues.
        assert_eq!(
            game.state.phase,
            GamePhase::Idle,
            "Game should continue when the opponent still has non-exempt pieces"
        );
    }

    // -----------------------------------------------------------------------
    // Test 5 — CHECK_COUNT: delivering the threshold check wins immediately
    //
    // Three-check: WHITE already has 2 checks delivered.
    // WHITE moves ROOK from [5,1] to [4,1], putting BLACK KING at [4,7] in check.
    // Count reaches 3 → WHITE wins.
    // -----------------------------------------------------------------------

    #[test]
    fn test_check_count_win_at_threshold() {
        let mut game = load_three_check();
        game.state.pieces.clear();

        insert(&mut game, vec![0, 0], "KING", "WHITE");
        insert(&mut game, vec![5, 1], "ROOK", "WHITE");
        insert(&mut game, vec![4, 7], "KING", "BLACK");

        // Pre-load 2 checks already delivered by WHITE.
        game.state.check_counts.insert("WHITE".to_string(), 2);

        // WHITE ROOK moves to [4,1] — same file as BLACK KING at [4,7].
        game.transition(GameTransition::CalculateMoves {
            position: vec![5, 1],
        })
        .unwrap();
        assert!(
            game.state
                .available_moves
                .as_ref()
                .unwrap()
                .contains_key(&vec![4u8, 1u8]),
            "WHITE ROOK should be able to move to [4,1]"
        );
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 1],
        })
        .unwrap();

        assert_eq!(
            game.state.phase,
            GamePhase::GameOver {
                winner: Some("WHITE".to_string())
            },
            "WHITE should win upon delivering the 3rd check"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6 — CHECK_COUNT: count increments but game continues below threshold
    //
    // Three-check with default count (0). WHITE delivers first check.
    // Count becomes 1, which is below the threshold of 3 → game continues.
    // -----------------------------------------------------------------------

    #[test]
    fn test_check_count_increments_without_winning() {
        let mut game = load_three_check();
        game.state.pieces.clear();

        insert(&mut game, vec![0, 0], "KING", "WHITE");
        insert(&mut game, vec![5, 1], "ROOK", "WHITE");
        insert(&mut game, vec![4, 7], "KING", "BLACK");

        // WHITE ROOK moves to [4,1] — delivers first check.
        game.transition(GameTransition::CalculateMoves {
            position: vec![5, 1],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 1],
        })
        .unwrap();

        assert_eq!(
            game.state.phase,
            GamePhase::Idle,
            "Game should continue after only 1 check (threshold is 3)"
        );
        assert_eq!(
            game.state.check_counts.get("WHITE").copied().unwrap_or(0),
            1,
            "WHITE's check count should be 1 after the first delivered check"
        );
    }
}
