#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load() -> Game {
        parse_game_spec("./specs/atomic.json")
            .map(Game::from_spec)
            .expect("Failed to load atomic spec")
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
    fn test_atomic_spec_loads() {
        load();
    }

    // -----------------------------------------------------------------------
    // Explosion: capturing piece and adjacents are destroyed; pawns survive the blast
    // -----------------------------------------------------------------------

    #[test]
    fn test_capture_destroys_capturing_piece_and_target() {
        let mut game = load();
        game.state.pieces.clear();

        // WHITE rook captures BLACK rook; both should be destroyed.
        insert(&mut game, vec![4, 4], "ROOK", "WHITE");
        insert(&mut game, vec![4, 6], "ROOK", "BLACK");
        insert(&mut game, vec![0, 0], "KING", "WHITE");
        insert(&mut game, vec![7, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![4, 4],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 6],
        })
        .unwrap();

        assert!(
            game.state.pieces.get(&vec![4, 6]).is_none(),
            "target square must be empty after explosion"
        );
        assert!(
            game.state.pieces.get(&vec![4, 4]).is_none(),
            "source square must be empty (capturing rook also destroyed)"
        );
    }

    #[test]
    fn test_explosion_clears_adjacent_non_pawn_pieces() {
        let mut game = load();
        game.state.pieces.clear();

        // WHITE rook captures BLACK piece; BLACK knight adjacent to the target is also destroyed.
        insert(&mut game, vec![4, 4], "ROOK", "WHITE");
        insert(&mut game, vec![4, 6], "ROOK", "BLACK");
        insert(&mut game, vec![5, 6], "KNIGHT", "BLACK"); // adjacent to explosion center [4,6]
        insert(&mut game, vec![0, 0], "KING", "WHITE");
        insert(&mut game, vec![7, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![4, 4],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 6],
        })
        .unwrap();

        assert!(
            game.state.pieces.get(&vec![5, 6]).is_none(),
            "adjacent BLACK knight must be destroyed by the blast"
        );
    }

    #[test]
    fn test_explosion_does_not_destroy_adjacent_pawns() {
        let mut game = load();
        game.state.pieces.clear();

        // WHITE rook captures BLACK rook; BLACK pawn adjacent to target must survive.
        insert(&mut game, vec![4, 4], "ROOK", "WHITE");
        insert(&mut game, vec![4, 6], "ROOK", "BLACK");
        insert(&mut game, vec![5, 6], "PAWN", "BLACK"); // adjacent pawn — should be EXEMPT
        insert(&mut game, vec![0, 0], "KING", "WHITE");
        insert(&mut game, vec![7, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![4, 4],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 6],
        })
        .unwrap();

        assert!(
            game.state.pieces.get(&vec![5, 6]).is_some(),
            "adjacent pawn must survive the explosion (pawns are exempt from blast)"
        );
    }

    // -----------------------------------------------------------------------
    // King cannot capture (would self-destruct)
    // -----------------------------------------------------------------------

    #[test]
    fn test_king_cannot_capture() {
        let mut game = load();
        game.state.pieces.clear();

        // WHITE king adjacent to BLACK rook — capture must NOT be available.
        insert(&mut game, vec![4, 4], "KING", "WHITE");
        insert(&mut game, vec![5, 4], "ROOK", "BLACK");
        insert(&mut game, vec![0, 7], "KING", "BLACK");

        let dests = moves_for(&mut game, vec![4, 4]);

        assert!(
            !dests.contains(&vec![5, 4]),
            "king must not be able to capture (would explode itself)"
        );
    }

    // -----------------------------------------------------------------------
    // Winning by exploding the opponent's king
    // -----------------------------------------------------------------------

    #[test]
    fn test_exploding_opponent_king_wins_the_game() {
        let mut game = load();
        game.state.pieces.clear();

        // WHITE rook is one step away from BLACK king; capturing it directly (center = king) wins.
        insert(&mut game, vec![4, 4], "ROOK", "WHITE");
        insert(&mut game, vec![4, 7], "KING", "BLACK");
        insert(&mut game, vec![0, 0], "KING", "WHITE");

        game.transition(GameTransition::CalculateMoves {
            position: vec![4, 4],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 7],
        })
        .unwrap();

        assert!(
            matches!(game.state.phase, GamePhase::GameOver { winner: Some(ref w) } if w == "WHITE"),
            "exploding the opponent's king must win the game for WHITE"
        );
    }

    #[test]
    fn test_adjacent_explosion_kills_opponent_king() {
        let mut game = load();
        game.state.pieces.clear();

        // WHITE queen captures a BLACK pawn that is adjacent to the BLACK king.
        // The explosion (radius 1) reaches the king → BLACK loses.
        insert(&mut game, vec![3, 5], "QUEEN", "WHITE");
        insert(&mut game, vec![4, 6], "PAWN", "BLACK"); // captured piece (pawn, center of blast)
        insert(&mut game, vec![5, 7], "KING", "BLACK"); // adjacent to [4,6] → caught in blast
        insert(&mut game, vec![0, 0], "KING", "WHITE");

        game.transition(GameTransition::CalculateMoves {
            position: vec![3, 5],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 6],
        })
        .unwrap();

        assert!(
            matches!(game.state.phase, GamePhase::GameOver { winner: Some(ref w) } if w == "WHITE"),
            "explosion adjacent to opponent king must end the game with WHITE winning"
        );
    }

    // -----------------------------------------------------------------------
    // Self-explosion causes immediate loss
    // -----------------------------------------------------------------------

    #[test]
    fn test_self_explosion_loses_the_game() {
        let mut game = load();
        game.state.pieces.clear();

        // WHITE rook captures BLACK pawn at [4,5]; WHITE king is adjacent at [5,5].
        // The explosion kills the WHITE king → WHITE loses immediately.
        insert(&mut game, vec![4, 4], "ROOK", "WHITE");
        insert(&mut game, vec![5, 5], "KING", "WHITE"); // caught in the blast at [4,5]
        insert(&mut game, vec![4, 5], "PAWN", "BLACK"); // target
        insert(&mut game, vec![7, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![4, 4],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 5],
        })
        .unwrap();

        assert!(
            matches!(game.state.phase, GamePhase::GameOver { winner: Some(ref w) } if w == "BLACK"),
            "a player who explodes their own king must lose immediately"
        );
    }
}
