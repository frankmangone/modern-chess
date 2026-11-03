#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load(spec_path: &str) -> Game {
        parse_game_spec(spec_path)
            .map(Game::from_spec)
            .expect("Failed to load spec for e2e test")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    fn play_move(game: &mut Game, from: Vec<u8>, to: Vec<u8>) {
        game.transition(GameTransition::CalculateMoves {
            position: from.clone(),
        })
        .unwrap();
        let moves = game
            .state
            .available_moves
            .as_ref()
            .expect("calculate moves should produce a move map");
        assert!(
            moves.contains_key(&to),
            "expected {:?} -> {:?} to be legal",
            from,
            to
        );
        game.transition(GameTransition::ExecuteMove { position: to })
            .unwrap();
    }

    #[test]
    fn chess_three_ply_opening_reaches_expected_final_state() {
        let mut game = load("./specs/chess.json");

        // 1. e4 e5 2. Nf3
        play_move(&mut game, vec![4, 1], vec![4, 3]);
        play_move(&mut game, vec![4, 6], vec![4, 4]);
        play_move(&mut game, vec![6, 0], vec![5, 2]);

        let white_pawn = game
            .piece_at_position(&vec![4, 3])
            .expect("white pawn should be on e4");
        assert_eq!(white_pawn.code, "PAWN");
        assert_eq!(white_pawn.player, "WHITE");

        let black_pawn = game
            .piece_at_position(&vec![4, 4])
            .expect("black pawn should be on e5");
        assert_eq!(black_pawn.code, "PAWN");
        assert_eq!(black_pawn.player, "BLACK");

        let white_knight = game
            .piece_at_position(&vec![5, 2])
            .expect("white knight should be on f3");
        assert_eq!(white_knight.code, "KNIGHT");
        assert_eq!(white_knight.player, "WHITE");

        assert!(
            game.piece_at_position(&vec![4, 1]).is_none(),
            "e2 should be empty after e4"
        );
        assert!(
            game.piece_at_position(&vec![4, 6]).is_none(),
            "e7 should be empty after e5"
        );
        assert!(
            game.piece_at_position(&vec![6, 0]).is_none(),
            "g1 should be empty after Nf3"
        );

        assert_eq!(game.history().len(), 3, "history should record three plies");
        assert_eq!(game.current_player(), "BLACK");
        assert!(
            matches!(game.state.phase, GamePhase::Idle),
            "game should remain active after opening sequence"
        );
    }

    #[test]
    fn atomic_three_ply_sequence_reaches_expected_final_state() {
        let mut game = load("./specs/atomic.json");
        game.state.pieces.clear();

        insert(&mut game, vec![0, 0], "KING", "WHITE");
        insert(&mut game, vec![7, 7], "KING", "BLACK");
        insert(&mut game, vec![4, 0], "ROOK", "WHITE");
        insert(&mut game, vec![4, 7], "ROOK", "BLACK");

        // 1. R(4,0)->(4,4) ... R(4,7)->(4,6) 2. R(4,4)x(4,6) (explosion)
        play_move(&mut game, vec![4, 0], vec![4, 4]);
        play_move(&mut game, vec![4, 7], vec![4, 6]);
        play_move(&mut game, vec![4, 4], vec![4, 6]);

        assert!(
            game.piece_at_position(&vec![4, 6]).is_none(),
            "explosion center should be empty"
        );
        assert!(
            game.piece_at_position(&vec![4, 4]).is_none(),
            "capturing rook should also be removed by explosion"
        );

        let white_king = game
            .piece_at_position(&vec![0, 0])
            .expect("white king should remain");
        assert_eq!(white_king.code, "KING");
        assert_eq!(white_king.player, "WHITE");

        let black_king = game
            .piece_at_position(&vec![7, 7])
            .expect("black king should remain");
        assert_eq!(black_king.code, "KING");
        assert_eq!(black_king.player, "BLACK");

        assert_eq!(game.history().len(), 3, "history should record three plies");
        assert_eq!(game.current_player(), "BLACK");
        assert!(
            matches!(game.state.phase, GamePhase::GameOver { winner: None }),
            "atomic sequence should resolve to draw when both sides are bare kings"
        );
    }
}
