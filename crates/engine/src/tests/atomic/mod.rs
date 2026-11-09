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
    // A capture may not blow up the mover's own king
    // -----------------------------------------------------------------------

    #[test]
    fn test_capture_that_would_explode_own_king_is_not_offered() {
        let mut game = load();
        game.state.pieces.clear();

        // The WHITE rook could take the BLACK pawn at [4,5], but WHITE's king at [5,5]
        // stands in the blast.
        insert(&mut game, vec![4, 4], "ROOK", "WHITE");
        insert(&mut game, vec![5, 5], "KING", "WHITE");
        insert(&mut game, vec![4, 5], "PAWN", "BLACK");
        insert(&mut game, vec![7, 7], "KING", "BLACK");

        let destinations = moves_for(&mut game, vec![4, 4]);

        assert!(
            !destinations.contains(&vec![4, 5]),
            "taking the pawn would explode WHITE's own king: {destinations:?}"
        );
        assert!(
            destinations.contains(&vec![4, 3]),
            "ordinary rook moves are still offered: {destinations:?}"
        );
    }

    #[test]
    fn test_capture_that_would_explode_both_kings_is_not_offered() {
        let mut game = load();
        game.state.pieces.clear();

        // The pawn at [4,3] stands next to both kings, so taking it would destroy both.
        insert(&mut game, vec![4, 0], "ROOK", "WHITE");
        insert(&mut game, vec![3, 3], "KING", "WHITE");
        insert(&mut game, vec![5, 3], "KING", "BLACK");
        insert(&mut game, vec![4, 3], "PAWN", "BLACK");

        let destinations = moves_for(&mut game, vec![4, 0]);

        assert!(
            !destinations.contains(&vec![4, 3]),
            "a capture that explodes both kings is not allowed: {destinations:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Taking the enemy king ends the game, even while in check
    // -----------------------------------------------------------------------

    #[test]
    fn test_exploding_the_enemy_king_is_allowed_while_in_check() {
        let mut game = load();
        game.state.pieces.clear();

        // WHITE's king at [4,0] is attacked by the BLACK rook on the same file.
        insert(&mut game, vec![4, 0], "KING", "WHITE");
        insert(&mut game, vec![4, 7], "ROOK", "BLACK");
        // The knight takes the pawn at [6,7]; the blast reaches BLACK's king at [7,7].
        insert(&mut game, vec![5, 5], "KNIGHT", "WHITE");
        insert(&mut game, vec![6, 7], "PAWN", "BLACK");
        insert(&mut game, vec![7, 7], "KING", "BLACK");
        assert!(game.leader_in_check(), "the setup must put WHITE in check");

        let destinations = moves_for(&mut game, vec![5, 5]);

        assert!(
            destinations.contains(&vec![6, 7]),
            "winning on the spot beats answering the check: {destinations:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Kings side by side cannot give check to each other
    // -----------------------------------------------------------------------

    #[test]
    fn test_a_king_next_to_the_enemy_king_is_not_in_check() {
        let mut game = load();
        game.state.pieces.clear();

        // The BLACK rook looks along the rank at WHITE's king, but capturing it would also
        // explode BLACK's king, which stands right next to it.
        insert(&mut game, vec![4, 3], "KING", "WHITE");
        insert(&mut game, vec![4, 4], "KING", "BLACK");
        insert(&mut game, vec![0, 3], "ROOK", "BLACK");
        insert(&mut game, vec![7, 1], "PAWN", "WHITE");

        assert!(!game.leader_in_check(), "WHITE is not really in check");
        let destinations = moves_for(&mut game, vec![7, 1]);

        assert!(
            destinations.contains(&vec![7, 2]),
            "WHITE may play elsewhere: {destinations:?}"
        );
    }

    #[test]
    fn test_a_king_with_the_enemy_king_far_away_is_still_in_check() {
        let mut game = load();
        game.state.pieces.clear();

        // The same rook, but BLACK's king is out of the blast: the check is real.
        insert(&mut game, vec![4, 3], "KING", "WHITE");
        insert(&mut game, vec![7, 7], "KING", "BLACK");
        insert(&mut game, vec![0, 3], "ROOK", "BLACK");
        insert(&mut game, vec![7, 1], "PAWN", "WHITE");

        assert!(game.leader_in_check());
        let destinations = moves_for(&mut game, vec![7, 1]);

        assert!(
            !destinations.contains(&vec![7, 2]),
            "a move that ignores a real check is not allowed: {destinations:?}"
        );
    }

    // -----------------------------------------------------------------------
    // A pawn that captures on the last rank is destroyed, so there is nothing to promote
    // -----------------------------------------------------------------------

    #[test]
    fn test_a_pawn_capturing_on_the_last_rank_explodes_and_does_not_promote() {
        let mut game = load();
        game.state.pieces.clear();
        insert(&mut game, vec![0, 0], "KING", "WHITE");
        insert(&mut game, vec![7, 7], "KING", "BLACK");
        insert(&mut game, vec![4, 6], "PAWN", "WHITE");
        insert(&mut game, vec![3, 7], "ROOK", "BLACK");
        insert(&mut game, vec![0, 3], "PAWN", "WHITE"); // so that it is not just two kings

        game.transition(GameTransition::CalculateMoves {
            position: vec![4, 6],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![3, 7],
        })
        .unwrap();

        assert!(game.state.pieces.get(&vec![3, 7]).is_none(), "the pawn exploded");
        assert!(
            matches!(game.state.phase, GamePhase::Idle),
            "no promotion is offered for a pawn that is gone: {:?}",
            game.state.phase
        );
        assert_eq!(game.current_player(), "BLACK");
    }
}
