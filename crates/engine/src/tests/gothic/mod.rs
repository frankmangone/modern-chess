#[cfg(test)]
mod tests {
    use crate::logic::{Game, Piece};
    use crate::shared::Position;
    use crate::specs::parse_game_spec;
    use crate::tests::support::{go, moves_from, pick, promotion_options};

    fn load() -> Game {
        parse_game_spec("./specs/gothic.json")
            .map(Game::from_spec)
            .expect("Failed to load gothic spec")
    }

    fn insert(game: &mut Game, pos: Position, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    fn moves_for(game: &mut Game, pos: Position) -> Vec<Position> {
        moves_from(game, pos)
    }

    // -----------------------------------------------------------------------
    // Spec loads
    // -----------------------------------------------------------------------

    #[test]
    fn test_gothic_spec_loads() {
        load();
    }

    // -----------------------------------------------------------------------
    // Board is 10x8
    // -----------------------------------------------------------------------

    #[test]
    fn test_gothic_board_dimensions() {
        let game = load();
        assert_eq!(
            game.board.dimensions,
            vec![10u8, 8u8],
            "Gothic Chess must be 10x8"
        );
    }

    // -----------------------------------------------------------------------
    // Kingside castling: king [5,0]→[8,0], rook [9,0]→[7,0]
    // -----------------------------------------------------------------------

    #[test]
    fn test_gothic_kingside_castling() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![5, 0], "KING", "WHITE");
        insert(&mut game, pos![9, 0], "ROOK", "WHITE");
        insert(&mut game, pos![5, 7], "KING", "BLACK");

        let moves = moves_for(&mut game, pos![5, 0]);
        assert!(
            moves.contains(&pos![8, 0]),
            "king must be able to castle kingside 3 squares to [8,0]"
        );

        go(&mut game, pos![8, 0]).unwrap();

        assert_eq!(
            game.state.pieces.get(&pos![8, 0]).map(|p| p.code.as_str()),
            Some("KING"),
            "king must land on [8,0]"
        );
        assert_eq!(
            game.state.pieces.get(&pos![7, 0]).map(|p| p.code.as_str()),
            Some("ROOK"),
            "rook must land on [7,0] after kingside castle"
        );
        assert!(
            game.state.pieces.get(&pos![9, 0]).is_none(),
            "rook source [9,0] must be vacated"
        );
    }

    // -----------------------------------------------------------------------
    // Queenside castling: king [5,0]→[2,0], rook [0,0]→[3,0]
    // -----------------------------------------------------------------------

    #[test]
    fn test_gothic_queenside_castling() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![5, 0], "KING", "WHITE");
        insert(&mut game, pos![0, 0], "ROOK", "WHITE");
        insert(&mut game, pos![5, 7], "KING", "BLACK");

        let moves = moves_for(&mut game, pos![5, 0]);
        assert!(
            moves.contains(&pos![2, 0]),
            "king must be able to castle queenside 3 squares to [2,0]"
        );

        go(&mut game, pos![2, 0]).unwrap();

        assert_eq!(
            game.state.pieces.get(&pos![2, 0]).map(|p| p.code.as_str()),
            Some("KING"),
            "king must land on [2,0]"
        );
        assert_eq!(
            game.state.pieces.get(&pos![3, 0]).map(|p| p.code.as_str()),
            Some("ROOK"),
            "rook must land on [3,0] after queenside castle"
        );
        assert!(
            game.state.pieces.get(&pos![0, 0]).is_none(),
            "rook source [0,0] must be vacated"
        );
    }

    // -----------------------------------------------------------------------
    // Pawn promotes to Archbishop and Chancellor on rank 7
    // -----------------------------------------------------------------------

    #[test]
    fn test_gothic_pawn_promotes_to_compound_pieces() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![3, 6], "PAWN", "WHITE");
        insert(&mut game, pos![5, 7], "KING", "BLACK");

        pick(&mut game, pos![3, 6]).unwrap();
        go(&mut game, pos![3, 7]).unwrap();

        let options = promotion_options(&game);
        assert!(
            !options.is_empty(),
            "expected a promotion to be asked after pawn reaches rank 7"
        );
        assert!(
            options.contains(&"ARCHBISHOP".to_string()),
            "ARCHBISHOP must be a promotion option"
        );
        assert!(
            options.contains(&"CHANCELLOR".to_string()),
            "CHANCELLOR must be a promotion option"
        );
    }

    // -----------------------------------------------------------------------
    // Archbishop = bishop slides + knight jumps
    // -----------------------------------------------------------------------

    #[test]
    fn test_gothic_archbishop_moves() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![4, 4], "ARCHBISHOP", "WHITE");
        insert(&mut game, pos![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, pos![4, 4]);

        assert!(
            dests.contains(&pos![7, 7]),
            "archbishop must slide diagonally"
        );
        assert!(
            dests.contains(&pos![6, 5]),
            "archbishop must jump like knight [2,1]"
        );
        assert!(
            dests.contains(&pos![5, 6]),
            "archbishop must jump like knight [1,2]"
        );
    }

    // -----------------------------------------------------------------------
    // Chancellor = rook slides + knight jumps
    // -----------------------------------------------------------------------

    #[test]
    fn test_gothic_chancellor_moves() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![4, 4], "CHANCELLOR", "WHITE");
        insert(&mut game, pos![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, pos![4, 4]);

        assert!(dests.contains(&pos![9, 4]), "chancellor must slide right");
        assert!(dests.contains(&pos![0, 4]), "chancellor must slide left");
        assert!(
            dests.contains(&pos![6, 5]),
            "chancellor must jump like knight [2,1]"
        );
    }
}
