#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load() -> Game {
        parse_game_spec("./specs/gothic.json")
            .map(Game::from_spec)
            .expect("Failed to load gothic spec")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    fn moves_for(game: &mut Game, pos: Vec<u8>) -> Vec<Vec<u8>> {
        game.transition(GameTransition::CalculateMoves { position: pos })
            .unwrap();
        let mut dests: Vec<Vec<u8>> = game
            .state
            .available_moves
            .as_ref()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        dests.sort();
        dests
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

        insert(&mut game, vec![5, 0], "KING", "WHITE");
        insert(&mut game, vec![9, 0], "ROOK", "WHITE");
        insert(&mut game, vec![5, 7], "KING", "BLACK");

        let moves = moves_for(&mut game, vec![5, 0]);
        assert!(
            moves.contains(&vec![8, 0]),
            "king must be able to castle kingside 3 squares to [8,0]"
        );

        game.transition(GameTransition::ExecuteMove {
            position: vec![8, 0],
        })
        .unwrap();

        assert_eq!(
            game.state.pieces.get(&vec![8, 0]).map(|p| p.code.as_str()),
            Some("KING"),
            "king must land on [8,0]"
        );
        assert_eq!(
            game.state.pieces.get(&vec![7, 0]).map(|p| p.code.as_str()),
            Some("ROOK"),
            "rook must land on [7,0] after kingside castle"
        );
        assert!(
            game.state.pieces.get(&vec![9, 0]).is_none(),
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

        insert(&mut game, vec![5, 0], "KING", "WHITE");
        insert(&mut game, vec![0, 0], "ROOK", "WHITE");
        insert(&mut game, vec![5, 7], "KING", "BLACK");

        let moves = moves_for(&mut game, vec![5, 0]);
        assert!(
            moves.contains(&vec![2, 0]),
            "king must be able to castle queenside 3 squares to [2,0]"
        );

        game.transition(GameTransition::ExecuteMove {
            position: vec![2, 0],
        })
        .unwrap();

        assert_eq!(
            game.state.pieces.get(&vec![2, 0]).map(|p| p.code.as_str()),
            Some("KING"),
            "king must land on [2,0]"
        );
        assert_eq!(
            game.state.pieces.get(&vec![3, 0]).map(|p| p.code.as_str()),
            Some("ROOK"),
            "rook must land on [3,0] after queenside castle"
        );
        assert!(
            game.state.pieces.get(&vec![0, 0]).is_none(),
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

        insert(&mut game, vec![3, 6], "PAWN", "WHITE");
        insert(&mut game, vec![5, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![3, 6],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![3, 7],
        })
        .unwrap();

        let GamePhase::Transforming { ref options, .. } = game.state.phase else {
            panic!("expected Transforming phase after pawn reaches rank 7");
        };
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

        insert(&mut game, vec![4, 4], "ARCHBISHOP", "WHITE");
        insert(&mut game, vec![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![4, 4]);

        assert!(
            dests.contains(&vec![7, 7]),
            "archbishop must slide diagonally"
        );
        assert!(
            dests.contains(&vec![6, 5]),
            "archbishop must jump like knight [2,1]"
        );
        assert!(
            dests.contains(&vec![5, 6]),
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

        insert(&mut game, vec![4, 4], "CHANCELLOR", "WHITE");
        insert(&mut game, vec![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![4, 4]);

        assert!(dests.contains(&vec![9, 4]), "chancellor must slide right");
        assert!(dests.contains(&vec![0, 4]), "chancellor must slide left");
        assert!(
            dests.contains(&vec![6, 5]),
            "chancellor must jump like knight [2,1]"
        );
    }
}
