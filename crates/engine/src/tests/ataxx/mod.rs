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
}
