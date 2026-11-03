#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::logic::{Game, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load_game() -> Game {
        parse_game_spec("./src/tests/move_players/spec.json")
            .map(Game::from_spec)
            .expect("Failed to load move_players test spec")
    }

    fn insert(game: &mut Game, pos: [u8; 2], player: &str) {
        game.state
            .pieces
            .insert(pos.to_vec(), Piece::new("WALKER".to_string(), player.to_string()));
    }

    fn offered_moves(game: &mut Game, from: [u8; 2]) -> BTreeSet<Vec<u8>> {
        game.transition(GameTransition::CalculateMoves {
            position: from.to_vec(),
        })
        .expect("selecting the piece should succeed");
        game.state
            .available_moves
            .as_ref()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Move 1 is restricted to WHITE: WHITE's walker gets its sideways step, and so does
    /// nobody else.
    #[test]
    fn test_restricted_move_is_offered_to_the_listed_player() {
        let mut game = load_game();
        insert(&mut game, [4, 4], "WHITE");

        let moves = offered_moves(&mut game, [4, 4]);

        let expected: BTreeSet<Vec<u8>> = [vec![4u8, 5u8], vec![5u8, 4u8]].into_iter().collect();
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_restricted_move_is_not_offered_to_other_players() {
        let mut game = load_game();
        insert(&mut game, [0, 0], "WHITE"); // keeps WHITE alive; BLACK moves second
        insert(&mut game, [4, 4], "BLACK");
        game.state.current_turn = 1; // BLACK to move

        let moves = offered_moves(&mut game, [4, 4]);

        // Unrestricted move 0 only: BLACK's "forward" is down the board.
        let expected: BTreeSet<Vec<u8>> = [vec![4u8, 3u8]].into_iter().collect();
        assert_eq!(moves, expected);
    }

    /// Threats follow the same filter: the WHITE-only capture must not make BLACK's
    /// walker attack anything, while it does let WHITE's walker attack.
    #[test]
    fn test_restricted_move_does_not_create_threats_for_other_players() {
        let mut game = load_game();
        insert(&mut game, [4, 4], "WHITE");
        insert(&mut game, [7, 7], "BLACK");

        assert!(game.attacked_by("WHITE").contains(&vec![5u8, 4u8]));
        assert!(!game.attacked_by("BLACK").contains(&vec![6u8, 7u8]));
        assert!(!game.attacked_by("BLACK").contains(&vec![8u8, 7u8]));
    }
}
