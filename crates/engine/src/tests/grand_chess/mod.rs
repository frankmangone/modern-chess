#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load() -> Game {
        parse_game_spec("./specs/grand_chess.json")
            .map(Game::from_spec)
            .expect("Failed to load grand_chess spec")
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
    fn test_grand_chess_spec_loads() {
        load();
    }

    // -----------------------------------------------------------------------
    // Board is 10×10
    // -----------------------------------------------------------------------

    #[test]
    fn test_grand_chess_board_dimensions() {
        let game = load();
        assert_eq!(
            game.board.dimensions,
            vec![10u8, 10u8],
            "Grand Chess must be 10x10"
        );
    }

    // -----------------------------------------------------------------------
    // Marshal = rook slides + knight jumps
    // -----------------------------------------------------------------------

    #[test]
    fn test_marshal_slides_orthogonally() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "MARSHAL", "WHITE");
        insert(&mut game, vec![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![4, 4]);

        assert!(dests.contains(&vec![9, 4]), "marshal must slide right");
        assert!(dests.contains(&vec![0, 4]), "marshal must slide left");
        assert!(dests.contains(&vec![4, 9]), "marshal must slide up");
        assert!(
            dests.contains(&vec![4, 0]),
            "marshal must slide down (captures king)"
        );
    }

    #[test]
    fn test_marshal_jumps_like_knight() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "MARSHAL", "WHITE");
        insert(&mut game, vec![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![4, 4]);

        assert!(dests.contains(&vec![6, 5]), "marshal must jump [2,1]");
        assert!(dests.contains(&vec![2, 3]), "marshal must jump [-2,-1]");
        assert!(dests.contains(&vec![5, 6]), "marshal must jump [1,2]");
        assert!(dests.contains(&vec![3, 2]), "marshal must jump [-1,-2]");
    }

    // -----------------------------------------------------------------------
    // Cardinal = bishop slides + knight jumps
    // -----------------------------------------------------------------------

    #[test]
    fn test_cardinal_slides_diagonally() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "CARDINAL", "WHITE");
        insert(&mut game, vec![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![4, 4]);

        assert!(
            dests.contains(&vec![8, 8]),
            "cardinal must slide NE diagonal"
        );
        assert!(
            dests.contains(&vec![1, 1]),
            "cardinal must slide toward [1,1]"
        );
        assert!(
            dests.contains(&vec![7, 1]),
            "cardinal must slide SE diagonal"
        );
    }

    #[test]
    fn test_cardinal_jumps_like_knight() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "CARDINAL", "WHITE");
        insert(&mut game, vec![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![4, 4]);

        assert!(dests.contains(&vec![6, 5]), "cardinal must jump [2,1]");
        assert!(dests.contains(&vec![5, 6]), "cardinal must jump [1,2]");
        assert!(dests.contains(&vec![2, 3]), "cardinal must jump [-2,-1]");
    }

    // -----------------------------------------------------------------------
    // Pawns start at row 2 and promote at row 9
    // -----------------------------------------------------------------------

    #[test]
    fn test_grand_chess_pawn_promotes_at_last_rank() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![3, 8], "PAWN", "WHITE");
        insert(&mut game, vec![4, 8], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![3, 8],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![3, 9],
        })
        .unwrap();

        let GamePhase::Transforming { ref options, .. } = game.state.phase else {
            panic!("expected Transforming phase after pawn reaches rank 9");
        };
        assert!(
            options.contains(&"QUEEN".to_string()),
            "QUEEN must be a promotion option"
        );
        assert!(
            options.contains(&"MARSHAL".to_string()),
            "MARSHAL must be a promotion option"
        );
        assert!(
            options.contains(&"CARDINAL".to_string()),
            "CARDINAL must be a promotion option"
        );
        assert!(
            options.contains(&"ROOK".to_string()),
            "ROOK must be a promotion option"
        );
        assert!(
            options.contains(&"BISHOP".to_string()),
            "BISHOP must be a promotion option"
        );
        assert!(
            options.contains(&"KNIGHT".to_string()),
            "KNIGHT must be a promotion option"
        );
    }

    // -----------------------------------------------------------------------
    // No castling in Grand Chess — king has no 2-or-3-square move
    // -----------------------------------------------------------------------

    #[test]
    fn test_grand_chess_no_castling() {
        let mut game = load();
        game.state.pieces.clear();

        // King at [4,1] with rooks on the flanks, path clear
        insert(&mut game, vec![4, 1], "KING", "WHITE");
        insert(&mut game, vec![0, 0], "ROOK", "WHITE");
        insert(&mut game, vec![9, 0], "ROOK", "WHITE");
        insert(&mut game, vec![4, 8], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![4, 1]);

        // King can only move 1 square in each direction
        for dest in &dests {
            let dx = (dest[0] as i16 - 4).abs();
            let dy = (dest[1] as i16 - 1).abs();
            assert!(
                dx <= 1 && dy <= 1,
                "king must only move 1 square at a time in Grand Chess, got {:?}",
                dest
            );
        }
    }
}
