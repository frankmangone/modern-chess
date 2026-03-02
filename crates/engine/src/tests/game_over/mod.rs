#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load_game() -> Game {
        parse_game_spec("./src/tests/game_over/spec.json")
            .map(Game::from_spec)
            .expect("Failed to load game_over test spec")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, code: &str, player: &str) {
        game.state.pieces.insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    // -------------------------------------------------------------------------
    // Test 1 — Stalemate
    //
    // WHITE KING_PIECE at [0,0]. BLACK SLIDERs at [2,1] and [1,4].
    //
    // After BLACK moves its SLIDER from [1,4] to [1,3]:
    //   - SLIDER [2,1] going west (BLACK actual [-1,0]):  threatens [1,1], [0,1]
    //   - SLIDER [1,3] going south (BLACK actual [0,-1]): threatens [1,2], [1,1], [1,0]
    //
    // WHITE KING_PIECE at [0,0] has only three on-board targets: [0,1], [1,0], [1,1].
    // All three are in BLACK's attack set → NOT_ATTACKED condition blocks all moves.
    // King is NOT itself attacked → stalemate → GameOver { winner: None }.
    // -------------------------------------------------------------------------

    #[test]
    fn test_stalemate_results_in_game_over_draw() {
        let mut game = load_game();

        // Place pieces.
        insert(&mut game, vec![0, 0], "KING_PIECE", "WHITE");
        insert(&mut game, vec![2, 1], "SLIDER", "BLACK");
        insert(&mut game, vec![1, 4], "SLIDER", "BLACK");

        // Advance to BLACK's turn.
        game.state.current_turn = 1;

        // BLACK moves the slider at [1,4] south to [1,3].
        game.transition(GameTransition::CalculateMoves { position: vec![1, 4] }).unwrap();
        assert!(
            game.state.available_moves.as_ref().unwrap().contains_key(&vec![1u8, 3u8]),
            "BLACK SLIDER at [1,4] should be able to move to [1,3]"
        );
        game.transition(GameTransition::ExecuteMove { position: vec![1, 3] }).unwrap();

        // After this move it is WHITE's turn. WHITE king has no legal moves and is
        // not in check → stalemate.
        assert_eq!(
            game.state.phase,
            GamePhase::GameOver { winner: None },
            "Phase should be GameOver with no winner (stalemate)"
        );
    }

    // -------------------------------------------------------------------------
    // Test 2 — Checkmate
    //
    // WHITE KING_PIECE at [0,0]. BLACK SLIDERs at [2,1] and [3,0].
    //
    // After BLACK moves its SLIDER from [3,0] to [2,0]:
    //   - SLIDER [2,1] going west (BLACK actual [-1,0]):  threatens [1,1], [0,1]
    //   - SLIDER [2,0] going west (BLACK actual [-1,0]):  threatens [1,0], [0,0]
    //
    // WHITE KING_PIECE has no legal moves ([0,1], [1,0], [1,1] all attacked),
    // and the king at [0,0] IS attacked → checkmate → GameOver { winner: Some("BLACK") }.
    // -------------------------------------------------------------------------

    #[test]
    fn test_checkmate_results_in_game_over_black_wins() {
        let mut game = load_game();

        insert(&mut game, vec![0, 0], "KING_PIECE", "WHITE");
        insert(&mut game, vec![2, 1], "SLIDER", "BLACK");
        insert(&mut game, vec![3, 0], "SLIDER", "BLACK");

        // Advance to BLACK's turn.
        game.state.current_turn = 1;

        // BLACK moves the slider at [3,0] west to [2,0].
        game.transition(GameTransition::CalculateMoves { position: vec![3, 0] }).unwrap();
        assert!(
            game.state.available_moves.as_ref().unwrap().contains_key(&vec![2u8, 0u8]),
            "BLACK SLIDER at [3,0] should be able to move to [2,0]"
        );
        game.transition(GameTransition::ExecuteMove { position: vec![2, 0] }).unwrap();

        assert_eq!(
            game.state.phase,
            GamePhase::GameOver { winner: Some("BLACK".to_string()) },
            "Phase should be GameOver with BLACK as winner (checkmate)"
        );
    }

    // -------------------------------------------------------------------------
    // Test 3 — Game continues when legal moves exist
    //
    // WHITE KING_PIECE at [4,4], BLACK SLIDER at [7,7] (harmless).
    //
    // After WHITE moves its king from [4,4] to [5,4], it becomes BLACK's turn.
    // BLACK SLIDER at [7,7] has many available moves → game continues → Idle.
    // -------------------------------------------------------------------------

    #[test]
    fn test_game_continues_when_legal_moves_exist() {
        let mut game = load_game();

        insert(&mut game, vec![4, 4], "KING_PIECE", "WHITE");
        insert(&mut game, vec![7, 7], "SLIDER", "BLACK");

        // WHITE's turn (current_turn = 0 by default).
        game.transition(GameTransition::CalculateMoves { position: vec![4, 4] }).unwrap();
        assert!(
            game.state.available_moves.as_ref().unwrap().contains_key(&vec![5u8, 4u8]),
            "WHITE KING_PIECE at [4,4] should be able to move to [5,4]"
        );
        game.transition(GameTransition::ExecuteMove { position: vec![5, 4] }).unwrap();

        assert_eq!(
            game.state.phase,
            GamePhase::Idle,
            "Phase should remain Idle when the next player has legal moves"
        );
    }
}
