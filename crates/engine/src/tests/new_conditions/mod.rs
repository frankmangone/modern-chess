#[cfg(test)]
mod tests {
    use crate::logic::{Game, Piece};
    use crate::shared::Position;
    use crate::specs::parse_game_spec;
    use crate::tests::support::{moves_from, pick, targets};

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

    fn insert(game: &mut Game, pos: Position, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    // -----------------------------------------------------------------------
    // NOT ATTACKED at START (SOURCE_NOT_ATTACKED before tests) — chess castling while in check
    // is illegal
    //
    // WHITE KING at [4,0] is attacked by a BLACK ROOK at [4,5].
    // The king must not be able to castle (queenside step [-2,0] or kingside [2,0])
    // even though its first-move, empty-path, path-not-attacked and rook tests
    // would otherwise hold.
    // -----------------------------------------------------------------------

    #[test]
    fn test_source_not_attacked_blocks_castling_while_in_check() {
        let mut game = load_chess();
        game.state.pieces.clear();

        // White king on e1 = [4,0], attacked by a rook on e6 = [4,5].
        insert(&mut game, pos![4, 0], "KING", "WHITE");
        insert(&mut game, pos![0, 0], "ROOK", "WHITE"); // queenside rook, never moved
        insert(&mut game, pos![7, 0], "ROOK", "WHITE"); // kingside rook, never moved
        insert(&mut game, pos![4, 5], "ROOK", "BLACK");
        insert(&mut game, pos![7, 7], "KING", "BLACK");

        pick(&mut game, pos![4, 0]).unwrap();
        let moves = targets(&game);

        // Castle targets would be [2,0] (queenside) and [6,0] (kingside).
        assert!(
            !moves.contains(&pos![2u8, 0u8]),
            "Queenside castle should be blocked when king is in check"
        );
        assert!(
            !moves.contains(&pos![6u8, 0u8]),
            "Kingside castle should be blocked when king is in check"
        );
    }

    #[test]
    fn test_source_not_attacked_allows_castling_when_safe() {
        let mut game = load_chess();
        game.state.pieces.clear();

        // White king on e1 = [4,0], NOT attacked.
        // Rooks on both sides, path between them is empty.
        insert(&mut game, pos![4, 0], "KING", "WHITE");
        insert(&mut game, pos![0, 0], "ROOK", "WHITE"); // queenside rook
        insert(&mut game, pos![7, 0], "ROOK", "WHITE"); // kingside rook
        insert(&mut game, pos![7, 7], "KING", "BLACK");

        pick(&mut game, pos![4, 0]).unwrap();
        let moves = targets(&game);

        assert!(
            moves.contains(&pos![2u8, 0u8]),
            "Queenside castle should be available when king is safe"
        );
        assert!(
            moves.contains(&pos![6u8, 0u8]),
            "Kingside castle should be available when king is safe"
        );
    }

    // -----------------------------------------------------------------------
    // COUNT over a PATH (PATH_PIECE_COUNT before tests) — cannon in mini_xiangqi captures over
    // exactly one screen
    //
    // Cannon at [2,0]. Screen piece (SOLDIER) at [3,0]. Enemy GENERAL at [4,0].
    // Cannon should be able to capture [4,0] (exactly 1 screen).
    // -----------------------------------------------------------------------

    #[test]
    fn test_path_piece_count_cannon_captures_over_one_screen() {
        let mut game = load_xiangqi();
        game.state.pieces.clear();

        // RED cannon at [1,0], screen (RED SOLDIER) at [2,0], BLACK GENERAL at [3,0].
        insert(&mut game, pos![4, 4], "GENERAL", "RED"); // RED general safe
        insert(&mut game, pos![1, 0], "CANNON", "RED");
        insert(&mut game, pos![2, 0], "SOLDIER", "RED"); // screen
        insert(&mut game, pos![3, 0], "GENERAL", "BLACK"); // target

        pick(&mut game, pos![1, 0]).unwrap();
        let moves = targets(&game);

        assert!(
            moves.contains(&pos![3u8, 0u8]),
            "Cannon should capture the enemy GENERAL over exactly one screen piece"
        );
    }

    #[test]
    fn test_path_piece_count_cannon_cannot_capture_without_screen() {
        let mut game = load_xiangqi();
        game.state.pieces.clear();

        // RED cannon at [1,0], no screen, BLACK GENERAL at [3,0].
        insert(&mut game, pos![4, 4], "GENERAL", "RED");
        insert(&mut game, pos![1, 0], "CANNON", "RED");
        insert(&mut game, pos![3, 0], "GENERAL", "BLACK");

        pick(&mut game, pos![1, 0]).unwrap();
        let moves = targets(&game);

        assert!(
            !moves.contains(&pos![3u8, 0u8]),
            "Cannon should NOT capture without a screen piece"
        );
    }

    #[test]
    fn test_path_piece_count_cannon_cannot_capture_over_two_screens() {
        let mut game = load_xiangqi();
        game.state.pieces.clear();

        // RED cannon at [0,0], two screens at [1,0] and [2,0], BLACK GENERAL at [3,0].
        insert(&mut game, pos![4, 4], "GENERAL", "RED");
        insert(&mut game, pos![0, 0], "CANNON", "RED");
        insert(&mut game, pos![1, 0], "SOLDIER", "RED"); // screen 1
        insert(&mut game, pos![2, 0], "SOLDIER", "RED"); // screen 2
        insert(&mut game, pos![3, 0], "GENERAL", "BLACK");

        pick(&mut game, pos![0, 0]).unwrap();
        let moves = targets(&game);

        assert!(
            !moves.contains(&pos![3u8, 0u8]),
            "Cannon should NOT capture over two screen pieces"
        );
    }

    // -----------------------------------------------------------------------
    // Cannon screens: either colour may act as the screen
    // -----------------------------------------------------------------------

    /// mini_xiangqi: a BLACK soldier on [2,0] is the screen (an *enemy* piece), the BLACK
    /// general on [3,0] is the target. A cannon may capture it, but not the screen itself.
    #[test]
    fn test_mini_xiangqi_cannon_can_use_an_enemy_piece_as_screen() {
        let mut game = load_xiangqi();
        game.state.pieces.clear();

        insert(&mut game, pos![4, 4], "GENERAL", "RED"); // RED general safe
        insert(&mut game, pos![0, 0], "CANNON", "RED");
        insert(&mut game, pos![2, 0], "SOLDIER", "BLACK"); // enemy screen
        insert(&mut game, pos![3, 0], "GENERAL", "BLACK"); // target

        pick(&mut game, pos![0, 0]).unwrap();
        let moves = targets(&game);

        assert!(
            moves.contains(&pos![3u8, 0u8]),
            "Cannon should capture over an enemy screen"
        );
        assert!(
            !moves.contains(&pos![2u8, 0u8]),
            "Cannon must not capture the screen itself: there is no screen in front of it"
        );
    }

    /// Janggi: an enemy chariot on [3,0] is the screen, a second enemy chariot on [5,0] is the
    /// target, and a third on [7,0] has two pieces in front of it, so it is out of reach.
    #[test]
    fn test_janggi_cannon_can_use_an_enemy_piece_as_screen() {
        let mut game = load_janggi();
        game.state.pieces.clear();

        insert(&mut game, pos![4, 1], "GENERAL", "CHO");
        insert(&mut game, pos![4, 8], "GENERAL", "HAN");
        insert(&mut game, pos![0, 0], "CANNON", "CHO");
        insert(&mut game, pos![3, 0], "CHARIOT", "HAN"); // enemy screen
        insert(&mut game, pos![5, 0], "CHARIOT", "HAN"); // target
        insert(&mut game, pos![7, 0], "CHARIOT", "HAN"); // behind two pieces

        pick(&mut game, pos![0, 0]).unwrap();
        let moves = targets(&game);

        assert!(
            moves.contains(&pos![5u8, 0u8]),
            "Cannon should capture over an enemy screen"
        );
        assert!(
            !moves.contains(&pos![3u8, 0u8]),
            "Cannon must not capture the screen itself"
        );
        assert!(
            !moves.contains(&pos![7u8, 0u8]),
            "Cannon must not capture over two pieces"
        );
    }

    // -----------------------------------------------------------------------
    // COUNT of cannons over a PATH — janggi cannon cannot use a cannon as its screen
    // -----------------------------------------------------------------------

    #[test]
    fn test_piece_not_at_cannon_cannot_use_cannon_as_screen() {
        let mut game = load_janggi();
        game.state.pieces.clear();

        // CHO cannon at [0,0], CHO cannon at [3,0] (another cannon as screen),
        // HAN chariot at [5,0] (target).
        insert(&mut game, pos![4, 1], "GENERAL", "CHO");
        insert(&mut game, pos![4, 8], "GENERAL", "HAN");
        insert(&mut game, pos![0, 0], "CANNON", "CHO");
        insert(&mut game, pos![3, 0], "CANNON", "CHO"); // cannon screen — illegal
        insert(&mut game, pos![5, 0], "CHARIOT", "HAN");

        pick(&mut game, pos![0, 0]).unwrap();
        let moves = targets(&game);

        assert!(
            !moves.contains(&pos![5u8, 0u8]),
            "Cannon should NOT be able to capture using another cannon as screen (PIECE_NOT_AT)"
        );
    }

    #[test]
    fn test_piece_not_at_cannon_can_use_chariot_as_screen() {
        let mut game = load_janggi();
        game.state.pieces.clear();

        // CHO cannon at [0,0], CHO chariot at [3,0] (valid screen),
        // HAN chariot at [5,0] (target).
        insert(&mut game, pos![4, 1], "GENERAL", "CHO");
        insert(&mut game, pos![4, 8], "GENERAL", "HAN");
        insert(&mut game, pos![0, 0], "CANNON", "CHO");
        insert(&mut game, pos![3, 0], "CHARIOT", "CHO"); // chariot screen — legal
        insert(&mut game, pos![5, 0], "CHARIOT", "HAN");

        pick(&mut game, pos![0, 0]).unwrap();
        let moves = targets(&game);

        assert!(
            moves.contains(&pos![5u8, 0u8]),
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
        insert(&mut game, pos![0, 0], "KING", "WHITE");
        insert(&mut game, pos![4, 1], "ROOK", "WHITE");
        insert(&mut game, pos![4, 7], "KING", "BLACK");

        pick(&mut game, pos![4, 1]).unwrap();
        let moves = targets(&game);

        // Moving ROOK to [4,2] through [4,6] would leave BLACK KING at [4,7] in check.
        for rank in 2u8..=6 {
            assert!(
                !moves.contains(&pos![4u8, rank]),
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
        insert(&mut game, pos![0, 0], "KING", "WHITE");
        insert(&mut game, pos![4, 1], "ROOK", "WHITE");
        insert(&mut game, pos![7, 7], "KING", "BLACK");

        pick(&mut game, pos![4, 1]).unwrap();
        let moves = targets(&game);

        // ROOK can move along file 4 without threatening [7,7].
        assert!(
            moves.contains(&pos![4u8, 2u8]),
            "ROOK should be able to move to [4,2] when BLACK KING is not on file 4"
        );
    }

    // -----------------------------------------------------------------------
    // COUNT of allies AROUND START (ALLY_ADJACENT_COUNT before tests) — solidarity: isolated
    // piece cannot move
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
        insert(&mut game, pos![0, 0], "KING", "WHITE");
        insert(&mut game, pos![4, 4], "ROOK", "WHITE");
        insert(&mut game, pos![7, 7], "KING", "BLACK");

        pick(&mut game, pos![4, 4]).unwrap();

        assert!(
            targets(&game).is_empty(),
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
        insert(&mut game, pos![0, 0], "KING", "WHITE");
        insert(&mut game, pos![4, 4], "ROOK", "WHITE");
        insert(&mut game, pos![4, 5], "QUEEN", "WHITE"); // adjacent ally
        insert(&mut game, pos![7, 7], "KING", "BLACK");

        pick(&mut game, pos![4, 4]).unwrap();
        let moves = targets(&game);

        assert!(
            !moves.is_empty(),
            "ROOK with at least one adjacent ally should have legal moves"
        );
    }

    /// The squares `from`'s piece can move to, for the player to move.
    fn destinations(game: &mut Game, from: Position) -> Vec<Position> {
        moves_from(game, from)
    }

    // -----------------------------------------------------------------------
    // NOT COUNT in front (DEPENDS_ON before tests) — a pawn's double step needs the square in
    // front to be empty
    // -----------------------------------------------------------------------

    #[test]
    fn test_pawn_double_step_needs_the_square_in_front_empty() {
        let setup = |blocker: Option<Position>| {
            let mut game = load_chess();
            game.state.pieces.clear();
            insert(&mut game, pos![0, 0], "KING", "WHITE");
            insert(&mut game, pos![7, 7], "KING", "BLACK");
            insert(&mut game, pos![4, 1], "PAWN", "WHITE");
            if let Some(at) = blocker {
                insert(&mut game, at, "KNIGHT", "BLACK");
            }
            game
        };

        let mut open = setup(None);
        assert_eq!(
            destinations(&mut open, pos![4, 1]),
            vec![pos![4, 2], pos![4, 3]]
        );

        let mut in_front = setup(Some(pos![4, 2]));
        assert_eq!(
            destinations(&mut in_front, pos![4, 1]),
            Vec::<Position>::new(),
            "a piece right in front stops both steps"
        );

        let mut on_landing = setup(Some(pos![4, 3]));
        assert_eq!(destinations(&mut on_landing, pos![4, 1]), vec![pos![4, 2]]);
    }

    #[test]
    fn test_black_pawn_double_step_needs_the_square_in_front_empty() {
        let mut game = load_chess();
        game.state.pieces.clear();
        insert(&mut game, pos![0, 0], "KING", "WHITE");
        insert(&mut game, pos![7, 7], "KING", "BLACK");
        insert(&mut game, pos![3, 6], "PAWN", "BLACK");
        insert(&mut game, pos![3, 5], "KNIGHT", "WHITE");
        game.state.current_turn = 1;
        assert_eq!(game.current_player(), "BLACK");

        assert_eq!(destinations(&mut game, pos![3, 6]), Vec::<Position>::new());
    }

    #[test]
    fn test_solidarity_pawn_double_step_needs_the_square_in_front_empty() {
        let setup = |blocker: Option<Position>| {
            let mut game = load_solidarity();
            game.state.pieces.clear();
            insert(&mut game, pos![0, 0], "KING", "WHITE");
            insert(&mut game, pos![7, 7], "KING", "BLACK");
            insert(&mut game, pos![4, 1], "PAWN", "WHITE");
            insert(&mut game, pos![5, 1], "PAWN", "WHITE"); // the ally the move needs
            if let Some(at) = blocker {
                insert(&mut game, at, "KNIGHT", "BLACK");
            }
            game
        };

        let mut open = setup(None);
        assert_eq!(
            destinations(&mut open, pos![4, 1]),
            vec![pos![4, 2], pos![4, 3]]
        );

        let mut in_front = setup(Some(pos![4, 2]));
        assert_eq!(
            destinations(&mut in_front, pos![4, 1]),
            Vec::<Position>::new()
        );

        let mut alone = load_solidarity();
        alone.state.pieces.clear();
        insert(&mut alone, pos![0, 0], "KING", "WHITE");
        insert(&mut alone, pos![7, 7], "KING", "BLACK");
        insert(&mut alone, pos![4, 1], "PAWN", "WHITE");
        assert_eq!(
            destinations(&mut alone, pos![4, 1]),
            Vec::<Position>::new(),
            "without an adjacent ally neither step is offered"
        );
    }

    // -----------------------------------------------------------------------
    // janggi: the cannon's quiet slide runs over empty squares and stops before any piece, a
    // cannon included. The slide's not-a-cannon test (PIECE_NOT_AT before tests) can never
    // fail here: a quiet move lands only on an empty square, where no cannon stands. What this
    // checks is where the slide stops, which its `until` decides.
    // -----------------------------------------------------------------------

    #[test]
    fn test_janggi_cannon_quiet_slide_stops_before_any_piece() {
        let mut game = load_janggi();
        game.state.pieces.clear();
        insert(&mut game, pos![3, 1], "GENERAL", "CHO");
        insert(&mut game, pos![5, 8], "GENERAL", "HAN");
        insert(&mut game, pos![0, 3], "CANNON", "CHO");
        insert(&mut game, pos![0, 6], "CANNON", "HAN");
        insert(&mut game, pos![3, 3], "CANNON", "CHO");

        assert_eq!(
            destinations(&mut game, pos![0, 3]),
            vec![
                pos![0, 0],
                pos![0, 1],
                pos![0, 2],
                pos![0, 4],
                pos![0, 5],
                pos![1, 3],
                pos![2, 3],
            ]
        );
    }
}
