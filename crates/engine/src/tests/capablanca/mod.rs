#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load() -> Game {
        parse_game_spec("./specs/capablanca.json")
            .map(Game::from_spec)
            .expect("Failed to load capablanca spec")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    fn moves_for(game: &mut Game, pos: Vec<u8>) -> Vec<Vec<u8>> {
        game.transition(GameTransition::CalculateMoves { position: pos }).unwrap();
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
    fn test_capablanca_spec_loads() {
        load();
    }

    // -----------------------------------------------------------------------
    // Archbishop = bishop slides + knight jumps
    // -----------------------------------------------------------------------

    #[test]
    fn test_archbishop_slides_diagonally() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "ARCHBISHOP", "WHITE");
        insert(&mut game, vec![0, 0], "KING", "BLACK"); // keep BLACK alive

        let dests = moves_for(&mut game, vec![4, 4]);

        // Should reach far diagonal squares (sliding)
        assert!(dests.contains(&vec![7, 7]), "archbishop must slide diagonally");
        assert!(dests.contains(&vec![0, 0]), "archbishop must be able to capture on diagonal");
        assert!(dests.contains(&vec![1, 1]), "archbishop must reach intermediate diagonals");
    }

    #[test]
    fn test_archbishop_jumps_like_knight() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "ARCHBISHOP", "WHITE");
        insert(&mut game, vec![0, 0], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![4, 4]);

        // Knight moves from [4,4]: [6,5], [6,3], [2,5], [2,3], [5,6], [5,2], [3,6], [3,2]
        assert!(dests.contains(&vec![6, 5]), "archbishop must jump knight-style [2,1]");
        assert!(dests.contains(&vec![6, 3]), "archbishop must jump knight-style [2,-1]");
        assert!(dests.contains(&vec![5, 6]), "archbishop must jump knight-style [1,2]");
        assert!(dests.contains(&vec![3, 2]), "archbishop must jump knight-style [-1,-2]");
    }

    #[test]
    fn test_archbishop_blocked_diagonally_but_not_for_knight_jump() {
        let mut game = load();
        game.state.pieces.clear();

        // Ally on the first diagonal square blocks the slide but not the knight jump
        insert(&mut game, vec![4, 4], "ARCHBISHOP", "WHITE");
        insert(&mut game, vec![5, 5], "PAWN",        "WHITE"); // blocks NE diagonal
        insert(&mut game, vec![0, 0], "KING",         "BLACK");

        let dests = moves_for(&mut game, vec![4, 4]);

        // Slide NE blocked at [5,5] — cannot reach [6,6], [7,7], etc.
        assert!(!dests.contains(&vec![6, 6]), "diagonal blocked by ally pawn");
        // But knight jump [2,1] is independent of the blocker
        assert!(dests.contains(&vec![6, 5]), "knight jump must still be available");
    }

    // -----------------------------------------------------------------------
    // Chancellor = rook slides + knight jumps
    // -----------------------------------------------------------------------

    #[test]
    fn test_chancellor_slides_orthogonally() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "CHANCELLOR", "WHITE");
        insert(&mut game, vec![0, 0], "KING",       "BLACK");

        let dests = moves_for(&mut game, vec![4, 4]);

        assert!(dests.contains(&vec![9, 4]), "chancellor must slide right");
        assert!(dests.contains(&vec![0, 4]), "chancellor must slide left");
        assert!(dests.contains(&vec![4, 9]), "chancellor must slide up");
        assert!(dests.contains(&vec![4, 0]), "chancellor must slide down");
    }

    #[test]
    fn test_chancellor_jumps_like_knight() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "CHANCELLOR", "WHITE");
        insert(&mut game, vec![0, 0], "KING",       "BLACK");

        let dests = moves_for(&mut game, vec![4, 4]);

        assert!(dests.contains(&vec![6, 5]), "chancellor must jump [2,1]");
        assert!(dests.contains(&vec![2, 3]), "chancellor must jump [-2,-1]");
        assert!(dests.contains(&vec![5, 6]), "chancellor must jump [1,2]");
        assert!(dests.contains(&vec![3, 2]), "chancellor must jump [-1,-2]");
    }

    // -----------------------------------------------------------------------
    // Pawn promotes to Archbishop and Chancellor
    // -----------------------------------------------------------------------

    #[test]
    fn test_pawn_can_promote_to_archbishop() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![3, 8], "PAWN", "WHITE");
        insert(&mut game, vec![5, 9], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves { position: vec![3, 8] }).unwrap();
        game.transition(GameTransition::ExecuteMove  { position: vec![3, 9] }).unwrap();

        let GamePhase::Transforming { ref options, .. } = game.state.phase else {
            panic!("expected Transforming phase after pawn reaches rank 9");
        };
        assert!(options.contains(&"ARCHBISHOP".to_string()), "ARCHBISHOP must be a promotion option");
        assert!(options.contains(&"CHANCELLOR".to_string()), "CHANCELLOR must be a promotion option");
        assert!(options.contains(&"QUEEN".to_string()),      "QUEEN must be a promotion option");

        // Complete the promotion
        game.transition(GameTransition::Transform { target: "ARCHBISHOP".to_string() }).unwrap();
        assert_eq!(
            game.state.pieces.get(&vec![3, 9]).map(|p| p.code.as_str()),
            Some("ARCHBISHOP"),
            "pawn should be replaced by ARCHBISHOP"
        );
    }

    // -----------------------------------------------------------------------
    // Castling: kingside (step [2,0]) — king [5,0]→[7,0], rook [9,0]→[6,0]
    // -----------------------------------------------------------------------

    #[test]
    fn test_kingside_castling() {
        let mut game = load();
        game.state.pieces.clear();

        // King at [5,0], kingside rook at [9,0], path [6,0],[7,0],[8,0] must be empty
        insert(&mut game, vec![5, 0], "KING", "WHITE");
        insert(&mut game, vec![9, 0], "ROOK", "WHITE");
        insert(&mut game, vec![5, 9], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves { position: vec![5, 0] }).unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();
        assert!(moves.contains_key(&vec![7, 0]), "king must have kingside castle as a legal move");

        game.transition(GameTransition::ExecuteMove { position: vec![7, 0] }).unwrap();

        assert_eq!(
            game.state.pieces.get(&vec![7, 0]).map(|p| p.code.as_str()),
            Some("KING"),
            "king must land on [7,0]"
        );
        assert_eq!(
            game.state.pieces.get(&vec![6, 0]).map(|p| p.code.as_str()),
            Some("ROOK"),
            "rook must land on [6,0] after kingside castle"
        );
        assert!(game.state.pieces.get(&vec![9, 0]).is_none(), "rook source must be vacated");
    }

    // -----------------------------------------------------------------------
    // Castling: queenside (step [-2,0]) — king [5,0]→[3,0], rook [0,0]→[4,0]
    // -----------------------------------------------------------------------

    #[test]
    fn test_queenside_castling() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![5, 0], "KING", "WHITE");
        insert(&mut game, vec![0, 0], "ROOK", "WHITE");
        insert(&mut game, vec![5, 9], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves { position: vec![5, 0] }).unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();
        assert!(moves.contains_key(&vec![3, 0]), "king must have queenside castle as a legal move");

        game.transition(GameTransition::ExecuteMove { position: vec![3, 0] }).unwrap();

        assert_eq!(
            game.state.pieces.get(&vec![3, 0]).map(|p| p.code.as_str()),
            Some("KING"),
            "king must land on [3,0]"
        );
        assert_eq!(
            game.state.pieces.get(&vec![4, 0]).map(|p| p.code.as_str()),
            Some("ROOK"),
            "rook must land on [4,0] after queenside castle"
        );
        assert!(game.state.pieces.get(&vec![0, 0]).is_none(), "rook source must be vacated");
    }
}
