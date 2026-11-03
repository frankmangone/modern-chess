#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load() -> Game {
        parse_game_spec("./specs/checkers.json")
            .map(Game::from_spec)
            .expect("Failed to load checkers spec")
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
    fn test_checkers_spec_loads() {
        load();
    }

    // -----------------------------------------------------------------------
    // Board is 8×8
    // -----------------------------------------------------------------------

    #[test]
    fn test_checkers_board_dimensions() {
        let game = load();
        assert_eq!(game.board.dimensions, vec![8u8, 8u8]);
    }

    // -----------------------------------------------------------------------
    // MAN moves forward diagonally only (no backward moves)
    // -----------------------------------------------------------------------

    #[test]
    fn test_man_moves_forward_only() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "MAN", "RED");

        let dests = moves_for(&mut game, vec![4, 4]);

        // forward-right and forward-left for RED
        assert!(dests.contains(&vec![5, 5]), "man must reach [5,5]");
        assert!(dests.contains(&vec![3, 5]), "man must reach [3,5]");

        // no backward moves
        assert!(!dests.contains(&vec![5, 3]), "man must NOT move backward to [5,3]");
        assert!(!dests.contains(&vec![3, 3]), "man must NOT move backward to [3,3]");
    }

    // -----------------------------------------------------------------------
    // MAN captures diagonally forward (jump over enemy, land on empty)
    // -----------------------------------------------------------------------

    #[test]
    fn test_man_captures_by_jumping() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![3, 3], "MAN", "RED");
        insert(&mut game, vec![4, 4], "MAN", "BLACK"); // enemy to jump over

        let dests = moves_for(&mut game, vec![3, 3]);

        // jump over [4,4] and land on [5,5]
        assert!(
            dests.contains(&vec![5, 5]),
            "man must jump over enemy to [5,5]"
        );
    }

    #[test]
    fn test_man_cannot_jump_over_ally() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![3, 3], "MAN", "RED");
        insert(&mut game, vec![4, 4], "MAN", "RED"); // ally — cannot jump

        let dests = moves_for(&mut game, vec![3, 3]);

        assert!(
            !dests.contains(&vec![5, 5]),
            "man must NOT jump over ally"
        );
    }

    #[test]
    fn test_man_cannot_jump_to_occupied_landing() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![3, 3], "MAN", "RED");
        insert(&mut game, vec![4, 4], "MAN", "BLACK"); // enemy to jump over
        insert(&mut game, vec![5, 5], "MAN", "RED"); // occupied landing square

        let dests = moves_for(&mut game, vec![3, 3]);

        assert!(
            !dests.contains(&vec![5, 5]),
            "man must NOT land on occupied square"
        );
    }

    // -----------------------------------------------------------------------
    // Multi-jump: Jumping phase entered after a capture with continuations
    // -----------------------------------------------------------------------

    #[test]
    fn test_multi_jump_enters_jumping_phase() {
        let mut game = load();
        game.state.pieces.clear();

        // RED man at [3,1], two BLACK men forming a double-jump path
        insert(&mut game, vec![3, 1], "MAN", "RED");
        insert(&mut game, vec![4, 2], "MAN", "BLACK"); // first enemy
        insert(&mut game, vec![4, 4], "MAN", "BLACK"); // second enemy (after first jump lands at [5,3])

        // calculate moves for RED man
        game.transition(GameTransition::CalculateMoves {
            position: vec![3, 1],
        })
        .unwrap();

        // execute the first jump: land at [5,3]
        game.transition(GameTransition::ExecuteMove {
            position: vec![5, 3],
        })
        .unwrap();

        // should be in Jumping phase now (further capture available at [3,5] is wrong, let me redo)
        // After landing at [5,3], enemy at [4,4] can be jumped to [3,5]
        assert!(
            matches!(game.state.phase, GamePhase::Jumping { .. }),
            "game must enter Jumping phase when further captures exist after a jump"
        );

        // The locked position should be the landing square [5,3]
        let GamePhase::Jumping { ref position } = game.state.phase else {
            panic!("expected Jumping phase");
        };
        assert_eq!(position, &vec![5u8, 3u8], "locked position must be [5,3]");
    }

    #[test]
    fn test_multi_jump_completes_on_second_capture() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![3, 1], "MAN", "RED");
        insert(&mut game, vec![4, 2], "MAN", "BLACK"); // first enemy (will be captured)
        insert(&mut game, vec![4, 4], "MAN", "BLACK"); // second enemy (will be captured)
        insert(&mut game, vec![0, 7], "MAN", "BLACK"); // surviving BLACK piece so game continues

        // First jump: [3,1] over [4,2] → [5,3]
        game.transition(GameTransition::CalculateMoves {
            position: vec![3, 1],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![5, 3],
        })
        .unwrap();

        // Should be in Jumping phase
        assert!(matches!(game.state.phase, GamePhase::Jumping { .. }));

        // Second jump: [5,3] over [4,4] → [3,5]
        game.transition(GameTransition::ExecuteMove {
            position: vec![3, 5],
        })
        .unwrap();

        // No more jumps → turn advances → Idle
        assert!(
            matches!(game.state.phase, GamePhase::Idle),
            "game must return to Idle after last jump"
        );

        // Both BLACK men should be captured
        assert!(
            game.state.pieces.get(&vec![4, 2]).is_none(),
            "first captured man must be removed"
        );
        assert!(
            game.state.pieces.get(&vec![4, 4]).is_none(),
            "second captured man must be removed"
        );

        // RED man ends at [3,5]
        assert_eq!(
            game.state.pieces.get(&vec![3, 5]).map(|p| p.code.as_str()),
            Some("MAN"),
            "RED man must land at [3,5]"
        );

        // Turn passed to BLACK
        assert_eq!(game.current_player(), "BLACK");
    }

    // -----------------------------------------------------------------------
    // Mandatory capture: non-capture moves hidden when capture is available
    // -----------------------------------------------------------------------

    #[test]
    fn test_mandatory_capture_hides_regular_moves() {
        let mut game = load();
        game.state.pieces.clear();

        // RED man has both a regular move and a capture available
        insert(&mut game, vec![3, 3], "MAN", "RED");
        insert(&mut game, vec![4, 4], "MAN", "BLACK"); // enemy → jump to [5,5]
        // [2,4] is empty (regular move available too, but must be hidden)

        let dests = moves_for(&mut game, vec![3, 3]);

        // Regular move to [2,4] must be hidden
        assert!(
            !dests.contains(&vec![2, 4]),
            "regular move must be hidden when capture is available"
        );
        // Jump to [5,5] must be shown
        assert!(
            dests.contains(&vec![5, 5]),
            "capture jump must be shown"
        );
    }

    // -----------------------------------------------------------------------
    // KING moves in all 4 diagonal directions
    // -----------------------------------------------------------------------

    #[test]
    fn test_king_moves_in_all_directions() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "KING", "RED");

        let dests = moves_for(&mut game, vec![4, 4]);

        assert!(dests.contains(&vec![5, 5]), "king must reach [5,5]");
        assert!(dests.contains(&vec![3, 5]), "king must reach [3,5]");
        assert!(dests.contains(&vec![5, 3]), "king must reach [5,3]");
        assert!(dests.contains(&vec![3, 3]), "king must reach [3,3]");
    }

    #[test]
    fn test_king_captures_backward() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 4], "KING", "RED");
        insert(&mut game, vec![3, 3], "MAN", "BLACK"); // enemy behind

        let dests = moves_for(&mut game, vec![4, 4]);

        // King can jump backward over enemy at [3,3] to land at [2,2]
        assert!(
            dests.contains(&vec![2, 2]),
            "king must capture backward to [2,2]"
        );
    }

    // -----------------------------------------------------------------------
    // MAN promotes to KING at opponent's back rank
    // -----------------------------------------------------------------------

    #[test]
    fn test_man_promotes_to_king_at_back_rank() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![3, 6], "MAN", "RED");

        game.transition(GameTransition::CalculateMoves {
            position: vec![3, 6],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 7],
        })
        .unwrap();

        let GamePhase::Transforming { ref options, .. } = game.state.phase else {
            panic!("expected Transforming phase after reaching back rank");
        };
        assert_eq!(options, &vec!["KING".to_string()]);
    }
}
