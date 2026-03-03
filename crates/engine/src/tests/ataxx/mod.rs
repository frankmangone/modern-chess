#[cfg(test)]
mod tests {
    use crate::logic::{Game, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load_ataxx() -> Game {
        parse_game_spec("./specs/ataxx.json")
            .map(Game::from_spec)
            .expect("Failed to load ataxx spec")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new("STONE".to_string(), player.to_string()));
    }

    fn stone_at(game: &Game, pos: Vec<u8>) -> Option<&str> {
        game.state.pieces.get(&pos).map(|p| p.player.as_str())
    }

    // -----------------------------------------------------------------------
    // Spec loads
    // -----------------------------------------------------------------------

    #[test]
    fn test_ataxx_spec_loads() {
        load_ataxx(); // panics on error
    }

    // -----------------------------------------------------------------------
    // Clone: source stone remains in place
    // -----------------------------------------------------------------------

    #[test]
    fn test_clone_leaves_stone_at_source() {
        let mut game = load_ataxx();
        game.state.pieces.clear();

        insert(&mut game, vec![3, 3], "RED");
        insert(&mut game, vec![6, 6], "BLUE"); // keep BLUE alive

        // RED clones right: [3,3] → [4,3]
        game.transition(GameTransition::CalculateMoves {
            position: vec![3, 3],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 3],
        })
        .unwrap();

        assert_eq!(
            stone_at(&game, vec![3, 3]),
            Some("RED"),
            "source stone must remain"
        );
        assert_eq!(
            stone_at(&game, vec![4, 3]),
            Some("RED"),
            "clone must appear at target"
        );
    }

    // -----------------------------------------------------------------------
    // Clone: adjacent enemy stones are converted
    // -----------------------------------------------------------------------

    #[test]
    fn test_clone_converts_adjacent_enemies() {
        let mut game = load_ataxx();
        game.state.pieces.clear();

        // RED at [3,3], BLUE neighbours that should flip after RED clones to [4,3].
        // Neighbours of [4,3]: [3,2],[3,3],[3,4],[4,2],[4,4],[5,2],[5,3],[5,4]
        // [3,3] is the source (ally), so 7 potential converts.
        insert(&mut game, vec![3, 3], "RED");
        insert(&mut game, vec![3, 2], "BLUE"); // neighbour of [4,3]
        insert(&mut game, vec![5, 3], "BLUE"); // neighbour of [4,3]
        insert(&mut game, vec![6, 6], "BLUE"); // keep BLUE alive (not adjacent)

        game.transition(GameTransition::CalculateMoves {
            position: vec![3, 3],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 3],
        })
        .unwrap();

        assert_eq!(
            stone_at(&game, vec![3, 2]),
            Some("RED"),
            "[3,2] should be converted"
        );
        assert_eq!(
            stone_at(&game, vec![5, 3]),
            Some("RED"),
            "[5,3] should be converted"
        );
    }

    // -----------------------------------------------------------------------
    // Clone: ally stones are NOT converted
    // -----------------------------------------------------------------------

    #[test]
    fn test_clone_does_not_convert_allies() {
        let mut game = load_ataxx();
        game.state.pieces.clear();

        insert(&mut game, vec![3, 3], "RED");
        insert(&mut game, vec![5, 3], "RED"); // ally adjacent to clone target [4,3]
        insert(&mut game, vec![6, 6], "BLUE");

        game.transition(GameTransition::CalculateMoves {
            position: vec![3, 3],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 3],
        })
        .unwrap();

        assert_eq!(
            stone_at(&game, vec![5, 3]),
            Some("RED"),
            "ally must not be converted"
        );
    }

    // -----------------------------------------------------------------------
    // Jump: source vacates, no conversions
    // -----------------------------------------------------------------------

    #[test]
    fn test_jump_moves_stone_without_converting() {
        let mut game = load_ataxx();
        game.state.pieces.clear();

        // RED at [3,3] jumps to [5,3] (step [2,0]).
        insert(&mut game, vec![3, 3], "RED");
        insert(&mut game, vec![4, 3], "BLUE"); // adjacent to jump target, must NOT convert
        insert(&mut game, vec![6, 6], "BLUE");

        game.transition(GameTransition::CalculateMoves {
            position: vec![3, 3],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![5, 3],
        })
        .unwrap();

        assert!(
            stone_at(&game, vec![3, 3]).is_none(),
            "source must be vacated on a jump"
        );
        assert_eq!(
            stone_at(&game, vec![5, 3]),
            Some("RED"),
            "stone must appear at jump target"
        );
        assert_eq!(
            stone_at(&game, vec![4, 3]),
            Some("BLUE"),
            "BLUE adjacent to target must not convert on jump"
        );
    }

    // -----------------------------------------------------------------------
    // Clone at edge: off-board CONVERT targets are ignored
    // -----------------------------------------------------------------------

    #[test]
    fn test_clone_near_edge_ignores_offboard_convert_targets() {
        let mut game = load_ataxx();
        game.state.pieces.clear();

        insert(&mut game, vec![0, 0], "RED");
        insert(&mut game, vec![6, 6], "BLUE"); // keep opponent alive

        game.transition(GameTransition::CalculateMoves {
            position: vec![0, 0],
        })
        .unwrap();
        assert!(
            game.state
                .available_moves
                .as_ref()
                .unwrap()
                .contains_key(&vec![1u8, 0u8]),
            "Clone from [0,0] to [1,0] should be legal at board edge"
        );

        game.transition(GameTransition::ExecuteMove {
            position: vec![1, 0],
        })
        .unwrap();

        assert_eq!(
            stone_at(&game, vec![0, 0]),
            Some("RED"),
            "Source should remain occupied after clone"
        );
        assert_eq!(
            stone_at(&game, vec![1, 0]),
            Some("RED"),
            "Clone target should be occupied by RED"
        );
    }

    // -----------------------------------------------------------------------
    // End-to-end sequence: multiple turns and final-state verification
    // -----------------------------------------------------------------------

    #[test]
    fn test_three_ply_sequence_reaches_expected_final_state() {
        let mut game = load_ataxx();
        game.state.pieces.clear();

        // RED can clone+convert on ply 1; BLUE has a far stone to move on ply 2.
        insert(&mut game, vec![3, 3], "RED");
        insert(&mut game, vec![5, 3], "BLUE");
        insert(&mut game, vec![6, 6], "BLUE");

        // Ply 1 (RED): clone [3,3] -> [4,3], converting adjacent BLUE at [5,3].
        game.transition(GameTransition::CalculateMoves {
            position: vec![3, 3],
        })
        .unwrap();
        assert!(
            game.state
                .available_moves
                .as_ref()
                .unwrap()
                .contains_key(&vec![4u8, 3u8]),
            "RED clone to [4,3] should be available"
        );
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 3],
        })
        .unwrap();

        // Ply 2 (BLUE): jump [6,6] -> [4,6] (jump has no COPY_SOURCE/CONVERT side effects).
        game.transition(GameTransition::CalculateMoves {
            position: vec![6, 6],
        })
        .unwrap();
        assert!(
            game.state
                .available_moves
                .as_ref()
                .unwrap()
                .contains_key(&vec![4u8, 6u8]),
            "BLUE jump to [4,6] should be available"
        );
        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 6],
        })
        .unwrap();

        // Ply 3 (RED): jump [4,3] -> [6,3], vacating [4,3].
        game.transition(GameTransition::CalculateMoves {
            position: vec![4, 3],
        })
        .unwrap();
        assert!(
            game.state
                .available_moves
                .as_ref()
                .unwrap()
                .contains_key(&vec![6u8, 3u8]),
            "RED jump to [6,3] should be available"
        );
        game.transition(GameTransition::ExecuteMove {
            position: vec![6, 3],
        })
        .unwrap();

        // Final state assertions.
        assert_eq!(stone_at(&game, vec![3, 3]), Some("RED"));
        assert_eq!(stone_at(&game, vec![5, 3]), Some("RED"));
        assert_eq!(stone_at(&game, vec![6, 3]), Some("RED"));
        assert!(stone_at(&game, vec![4, 3]).is_none());

        assert_eq!(stone_at(&game, vec![4, 6]), Some("BLUE"));
        assert!(stone_at(&game, vec![6, 6]).is_none());

        let red_count = game
            .state
            .pieces
            .values()
            .filter(|p| p.player == "RED")
            .count();
        let blue_count = game
            .state
            .pieces
            .values()
            .filter(|p| p.player == "BLUE")
            .count();
        assert_eq!(red_count, 3, "Expected three RED stones after sequence");
        assert_eq!(blue_count, 1, "Expected one BLUE stone after sequence");
        assert_eq!(game.state.history.len(), 3, "Expected three executed plies");
    }
}
