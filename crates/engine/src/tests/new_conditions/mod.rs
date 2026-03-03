#[cfg(test)]
mod tests {
    use crate::logic::{Game, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load_chess() -> Game {
        parse_game_spec("./specs/chess.json")
            .map(Game::from_spec)
            .expect("Failed to load chess spec")
    }

    fn load_xiangqi() -> Game {
        parse_game_spec("./specs/mini_xiangqi.json")
            .map(Game::from_spec)
            .expect("Failed to load mini_xiangqi spec")
    }

    fn load_janggi() -> Game {
        parse_game_spec("./specs/janggi_cannon.json")
            .map(Game::from_spec)
            .expect("Failed to load janggi_cannon spec")
    }

    fn load_racing() -> Game {
        parse_game_spec("./specs/racing_kings.json")
            .map(Game::from_spec)
            .expect("Failed to load racing_kings spec")
    }

    fn load_solidarity() -> Game {
        parse_game_spec("./specs/solidarity.json")
            .map(Game::from_spec)
            .expect("Failed to load solidarity spec")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    // -----------------------------------------------------------------------
    // SOURCE_NOT_ATTACKED — chess castling while in check is now illegal
    //
    // WHITE KING at [4,0] is attacked by a BLACK ROOK at [4,5].
    // The king must not be able to castle (queenside step [-2,0] or kingside [2,0])
    // even though FIRST_MOVE + PATH_EMPTY + PATH_NOT_ATTACKED + ROOK conditions
    // would otherwise be satisfied.
    // -----------------------------------------------------------------------

    #[test]
    fn test_source_not_attacked_blocks_castling_while_in_check() {
        let mut game = load_chess();
        game.state.pieces.clear();

        // White king on e1 = [4,0], attacked by a rook on e6 = [4,5].
        insert(&mut game, vec![4, 0], "KING", "WHITE");
        insert(&mut game, vec![0, 0], "ROOK", "WHITE"); // queenside rook, never moved
        insert(&mut game, vec![7, 0], "ROOK", "WHITE"); // kingside rook, never moved
        insert(&mut game, vec![4, 5], "ROOK", "BLACK");
        insert(&mut game, vec![7, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![4, 0],
        })
        .unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();

        // Castle targets would be [2,0] (queenside) and [6,0] (kingside).
        assert!(
            !moves.contains_key(&vec![2u8, 0u8]),
            "Queenside castle should be blocked when king is in check"
        );
        assert!(
            !moves.contains_key(&vec![6u8, 0u8]),
            "Kingside castle should be blocked when king is in check"
        );
    }

    #[test]
    fn test_source_not_attacked_allows_castling_when_safe() {
        let mut game = load_chess();
        game.state.pieces.clear();

        // White king on e1 = [4,0], NOT attacked.
        // Rooks on both sides, path between them is empty.
        insert(&mut game, vec![4, 0], "KING", "WHITE");
        insert(&mut game, vec![0, 0], "ROOK", "WHITE"); // queenside rook
        insert(&mut game, vec![7, 0], "ROOK", "WHITE"); // kingside rook
        insert(&mut game, vec![7, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![4, 0],
        })
        .unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();

        assert!(
            moves.contains_key(&vec![2u8, 0u8]),
            "Queenside castle should be available when king is safe"
        );
        assert!(
            moves.contains_key(&vec![6u8, 0u8]),
            "Kingside castle should be available when king is safe"
        );
    }

    // -----------------------------------------------------------------------
    // PATH_PIECE_COUNT — cannon in mini_xiangqi captures over exactly one screen
    //
    // Cannon at [2,0]. Screen piece (SOLDIER) at [3,0]. Enemy GENERAL at [4,0].
    // Cannon should be able to capture [4,0] (exactly 1 screen).
    // -----------------------------------------------------------------------

    #[test]
    fn test_path_piece_count_cannon_captures_over_one_screen() {
        let mut game = load_xiangqi();
        game.state.pieces.clear();

        // RED cannon at [1,0], screen (RED SOLDIER) at [2,0], BLACK GENERAL at [3,0].
        insert(&mut game, vec![4, 4], "GENERAL", "RED"); // RED leader safe
        insert(&mut game, vec![1, 0], "CANNON", "RED");
        insert(&mut game, vec![2, 0], "SOLDIER", "RED"); // screen
        insert(&mut game, vec![3, 0], "GENERAL", "BLACK"); // target

        game.transition(GameTransition::CalculateMoves {
            position: vec![1, 0],
        })
        .unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();

        assert!(
            moves.contains_key(&vec![3u8, 0u8]),
            "Cannon should capture the enemy GENERAL over exactly one screen piece"
        );
    }

    #[test]
    fn test_path_piece_count_cannon_cannot_capture_without_screen() {
        let mut game = load_xiangqi();
        game.state.pieces.clear();

        // RED cannon at [1,0], no screen, BLACK GENERAL at [3,0].
        insert(&mut game, vec![4, 4], "GENERAL", "RED");
        insert(&mut game, vec![1, 0], "CANNON", "RED");
        insert(&mut game, vec![3, 0], "GENERAL", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![1, 0],
        })
        .unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();

        assert!(
            !moves.contains_key(&vec![3u8, 0u8]),
            "Cannon should NOT capture without a screen piece"
        );
    }

    #[test]
    fn test_path_piece_count_cannon_cannot_capture_over_two_screens() {
        let mut game = load_xiangqi();
        game.state.pieces.clear();

        // RED cannon at [0,0], two screens at [1,0] and [2,0], BLACK GENERAL at [3,0].
        insert(&mut game, vec![4, 4], "GENERAL", "RED");
        insert(&mut game, vec![0, 0], "CANNON", "RED");
        insert(&mut game, vec![1, 0], "SOLDIER", "RED"); // screen 1
        insert(&mut game, vec![2, 0], "SOLDIER", "RED"); // screen 2
        insert(&mut game, vec![3, 0], "GENERAL", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![0, 0],
        })
        .unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();

        assert!(
            !moves.contains_key(&vec![3u8, 0u8]),
            "Cannon should NOT capture over two screen pieces"
        );
    }

    // -----------------------------------------------------------------------
    // PIECE_NOT_AT — janggi cannon cannot use a cannon as its screen
    // -----------------------------------------------------------------------

    #[test]
    fn test_piece_not_at_cannon_cannot_use_cannon_as_screen() {
        let mut game = load_janggi();
        game.state.pieces.clear();

        // CHO cannon at [0,0], CHO cannon at [3,0] (another cannon as screen),
        // HAN chariot at [5,0] (target).
        insert(&mut game, vec![4, 1], "GENERAL", "CHO");
        insert(&mut game, vec![4, 8], "GENERAL", "HAN");
        insert(&mut game, vec![0, 0], "CANNON", "CHO");
        insert(&mut game, vec![3, 0], "CANNON", "CHO"); // cannon screen — illegal
        insert(&mut game, vec![5, 0], "CHARIOT", "HAN");

        game.transition(GameTransition::CalculateMoves {
            position: vec![0, 0],
        })
        .unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();

        assert!(
            !moves.contains_key(&vec![5u8, 0u8]),
            "Cannon should NOT be able to capture using another cannon as screen (PIECE_NOT_AT)"
        );
    }

    #[test]
    fn test_piece_not_at_cannon_can_use_chariot_as_screen() {
        let mut game = load_janggi();
        game.state.pieces.clear();

        // CHO cannon at [0,0], CHO chariot at [3,0] (valid screen),
        // HAN chariot at [5,0] (target).
        insert(&mut game, vec![4, 1], "GENERAL", "CHO");
        insert(&mut game, vec![4, 8], "GENERAL", "HAN");
        insert(&mut game, vec![0, 0], "CANNON", "CHO");
        insert(&mut game, vec![3, 0], "CHARIOT", "CHO"); // chariot screen — legal
        insert(&mut game, vec![5, 0], "CHARIOT", "HAN");

        game.transition(GameTransition::CalculateMoves {
            position: vec![0, 0],
        })
        .unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();

        assert!(
            moves.contains_key(&vec![5u8, 0u8]),
            "Cannon should be able to capture using a non-cannon piece as screen"
        );
    }

    // -----------------------------------------------------------------------
    // OPPONENT_NOT_IN_CHECK — racing kings: move that gives check is illegal
    //
    // WHITE ROOK at [4,1]. BLACK KING at [4,7].
    // WHITE moving its ROOK to [4,2] would put BLACK KING in check via the same file.
    // That move must be filtered out by OPPONENT_NOT_IN_CHECK.
    // -----------------------------------------------------------------------

    #[test]
    fn test_opponent_not_in_check_blocks_check_giving_move() {
        let mut game = load_racing();
        game.state.pieces.clear();

        // WHITE KING somewhere safe, WHITE ROOK about to give check, BLACK KING on same file.
        insert(&mut game, vec![0, 0], "KING", "WHITE");
        insert(&mut game, vec![4, 1], "ROOK", "WHITE");
        insert(&mut game, vec![4, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![4, 1],
        })
        .unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();

        // Moving ROOK to [4,2] through [4,6] would leave BLACK KING at [4,7] in check.
        for rank in 2u8..=6 {
            assert!(
                !moves.contains_key(&vec![4u8, rank]),
                "ROOK should not be able to move to [4,{}] as it would give check",
                rank
            );
        }
    }

    #[test]
    fn test_opponent_not_in_check_allows_safe_moves() {
        let mut game = load_racing();
        game.state.pieces.clear();

        // WHITE KING at [0,0], WHITE ROOK at [4,1].
        // BLACK KING at [7,7] (different file and rank — no check possible via [4,x]).
        insert(&mut game, vec![0, 0], "KING", "WHITE");
        insert(&mut game, vec![4, 1], "ROOK", "WHITE");
        insert(&mut game, vec![7, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![4, 1],
        })
        .unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();

        // ROOK can move along file 4 without threatening [7,7].
        assert!(
            moves.contains_key(&vec![4u8, 2u8]),
            "ROOK should be able to move to [4,2] when BLACK KING is not on file 4"
        );
    }

    // -----------------------------------------------------------------------
    // ALLY_ADJACENT_COUNT — solidarity: isolated piece cannot move
    //
    // A ROOK completely isolated from allies (no adjacent ally pieces)
    // must not have any legal moves.
    // -----------------------------------------------------------------------

    #[test]
    fn test_ally_adjacent_count_isolated_piece_cannot_move() {
        let mut game = load_solidarity();
        game.state.pieces.clear();

        // WHITE ROOK isolated at [4,4]. No WHITE ally adjacent.
        // WHITE KING at [0,0] (not adjacent).
        // BLACK KING at [7,7] (far away, harmless).
        insert(&mut game, vec![0, 0], "KING", "WHITE");
        insert(&mut game, vec![4, 4], "ROOK", "WHITE");
        insert(&mut game, vec![7, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![4, 4],
        })
        .unwrap();

        assert!(
            game.state
                .available_moves
                .as_ref()
                .map_or(true, |m| m.is_empty()),
            "Isolated ROOK with no adjacent allies should have no legal moves"
        );
    }

    #[test]
    fn test_ally_adjacent_count_connected_piece_can_move() {
        let mut game = load_solidarity();
        game.state.pieces.clear();

        // WHITE ROOK at [4,4] with a WHITE QUEEN adjacent at [4,5].
        // WHITE KING at [0,0] (not adjacent to rook).
        // BLACK KING at [7,7].
        insert(&mut game, vec![0, 0], "KING", "WHITE");
        insert(&mut game, vec![4, 4], "ROOK", "WHITE");
        insert(&mut game, vec![4, 5], "QUEEN", "WHITE"); // adjacent ally
        insert(&mut game, vec![7, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![4, 4],
        })
        .unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();

        assert!(
            !moves.is_empty(),
            "ROOK with at least one adjacent ally should have legal moves"
        );
    }
}
