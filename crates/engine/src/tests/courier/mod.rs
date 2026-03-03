#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load() -> Game {
        parse_game_spec("./specs/courier.json")
            .map(Game::from_spec)
            .expect("Failed to load courier spec")
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
    fn test_courier_spec_loads() {
        load();
    }

    // -----------------------------------------------------------------------
    // Board is 12×8
    // -----------------------------------------------------------------------

    #[test]
    fn test_courier_board_dimensions() {
        let game = load();
        assert_eq!(game.board.dimensions, vec![12u8, 8u8], "Courier Chess must be 12x8");
    }

    // -----------------------------------------------------------------------
    // Elephant jumps exactly 2 squares diagonally (leaps over pieces)
    // -----------------------------------------------------------------------

    #[test]
    fn test_elephant_jumps_two_diagonally() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![5, 4], "ELEPHANT", "WHITE");
        insert(&mut game, vec![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![5, 4]);

        assert_eq!(dests.len(), 4, "elephant must have exactly 4 jump destinations");
        assert!(dests.contains(&vec![7, 6]), "elephant must reach [7,6]");
        assert!(dests.contains(&vec![7, 2]), "elephant must reach [7,2]");
        assert!(dests.contains(&vec![3, 6]), "elephant must reach [3,6]");
        assert!(dests.contains(&vec![3, 2]), "elephant must reach [3,2]");
    }

    #[test]
    fn test_elephant_leaps_over_pieces() {
        let mut game = load();
        game.state.pieces.clear();

        // Blocker on the diagonal between elephant and landing square
        insert(&mut game, vec![5, 4], "ELEPHANT", "WHITE");
        insert(&mut game, vec![6, 5], "PAWN", "WHITE"); // on the path NE
        insert(&mut game, vec![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![5, 4]);

        // Should still reach [7,6] despite pawn at [6,5]
        assert!(
            dests.contains(&vec![7, 6]),
            "elephant must leap over a piece on the diagonal"
        );
    }

    // -----------------------------------------------------------------------
    // Courier (bishop) slides diagonally
    // -----------------------------------------------------------------------

    #[test]
    fn test_courier_slides_diagonally() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![5, 4], "COURIER", "WHITE");
        insert(&mut game, vec![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![5, 4]);

        assert!(dests.contains(&vec![9, 0]), "courier must slide to far corner [9,0]");
        assert!(dests.contains(&vec![1, 0]), "courier must slide to [1,0] (captures king)");
        assert!(dests.contains(&vec![8, 7]), "courier must slide to [8,7]");
        assert!(dests.contains(&vec![6, 5]), "courier must reach [6,5]");
    }

    // -----------------------------------------------------------------------
    // Sage moves 1 step in any direction (non-royal)
    // -----------------------------------------------------------------------

    #[test]
    fn test_sage_moves_one_step_any_direction() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![5, 4], "SAGE", "WHITE");
        insert(&mut game, vec![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![5, 4]);

        assert_eq!(dests.len(), 8, "sage must have exactly 8 destinations from center");
        assert!(dests.contains(&vec![6, 4]), "sage must reach [6,4]");
        assert!(dests.contains(&vec![4, 4]), "sage must reach [4,4]");
        assert!(dests.contains(&vec![5, 5]), "sage must reach [5,5]");
        assert!(dests.contains(&vec![6, 5]), "sage must reach [6,5]");
    }

    // -----------------------------------------------------------------------
    // Ferz moves 1 step diagonally only
    // -----------------------------------------------------------------------

    #[test]
    fn test_ferz_moves_one_step_diagonally() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![5, 4], "FERZ", "WHITE");
        insert(&mut game, vec![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![5, 4]);

        assert_eq!(dests.len(), 4, "ferz must have exactly 4 destinations");
        assert!(dests.contains(&vec![6, 5]), "ferz must reach [6,5]");
        assert!(dests.contains(&vec![6, 3]), "ferz must reach [6,3]");
        assert!(dests.contains(&vec![4, 5]), "ferz must reach [4,5]");
        assert!(dests.contains(&vec![4, 3]), "ferz must reach [4,3]");
    }

    // -----------------------------------------------------------------------
    // Fool moves 1 step orthogonally only
    // -----------------------------------------------------------------------

    #[test]
    fn test_fool_moves_one_step_orthogonally() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![5, 4], "FOOL", "WHITE");
        insert(&mut game, vec![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![5, 4]);

        assert_eq!(dests.len(), 4, "fool must have exactly 4 destinations");
        assert!(dests.contains(&vec![6, 4]), "fool must reach [6,4]");
        assert!(dests.contains(&vec![4, 4]), "fool must reach [4,4]");
        assert!(dests.contains(&vec![5, 5]), "fool must reach [5,5]");
        assert!(dests.contains(&vec![5, 3]), "fool must reach [5,3]");
    }

    // -----------------------------------------------------------------------
    // Pawn: no double-step, promotes only to FERZ
    // -----------------------------------------------------------------------

    #[test]
    fn test_courier_pawn_no_double_step() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![5, 1], "PAWN", "WHITE");
        insert(&mut game, vec![5, 7], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![5, 1]);

        assert!(dests.contains(&vec![5, 2]), "pawn must be able to move forward");
        assert!(
            !dests.contains(&vec![5, 3]),
            "pawn must NOT be able to double-step in Courier Chess"
        );
    }

    #[test]
    fn test_courier_pawn_promotes_to_ferz_only() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![5, 6], "PAWN", "WHITE");
        insert(&mut game, vec![5, 7], "KING", "BLACK"); // enemy king on row 7

        game.transition(GameTransition::CalculateMoves {
            position: vec![5, 6],
        })
        .unwrap();
        // pawn can capture the king diagonally — use forward move instead
        // place king elsewhere
        game.state.pieces.clear();
        insert(&mut game, vec![5, 6], "PAWN", "WHITE");
        insert(&mut game, vec![0, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![5, 6],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![5, 7],
        })
        .unwrap();

        let GamePhase::Transforming { ref options, .. } = game.state.phase else {
            panic!("expected Transforming phase after pawn reaches rank 7");
        };
        assert_eq!(options.len(), 1, "pawn must only have one promotion option");
        assert!(
            options.contains(&"FERZ".to_string()),
            "pawn must promote only to FERZ in Courier Chess"
        );
        assert!(
            !options.contains(&"QUEEN".to_string()),
            "pawn must NOT promote to QUEEN in Courier Chess"
        );
    }
}
