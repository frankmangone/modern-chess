#[cfg(test)]
mod tests {
    use crate::shared::Position;
    use std::collections::BTreeSet;

    use crate::logic::{Game, Piece};
    use crate::specs::parse_game_spec;
    use crate::tests::support::{pick, targets};

    fn load_game() -> Game {
        parse_game_spec("./src/tests/move_players/spec.json")
            .map(Game::from_spec)
            .expect("Failed to load move_players test spec")
    }

    fn insert(game: &mut Game, pos: [u8; 2], player: &str) {
        game.state.pieces.insert(
            Position::from_coordinates(&pos),
            Piece::new("WALKER".to_string(), player.to_string()),
        );
    }

    fn offered_moves(game: &mut Game, from: [u8; 2]) -> BTreeSet<Position> {
        pick(game, Position::from_coordinates(&from)).expect("selecting the piece should succeed");
        targets(&game).into_iter().collect()
    }

    /// Move 1 is restricted to WHITE: WHITE's walker gets its sideways step, and so does
    /// nobody else.
    #[test]
    fn test_restricted_move_is_offered_to_the_listed_player() {
        let mut game = load_game();
        insert(&mut game, [4, 4], "WHITE");

        let moves = offered_moves(&mut game, [4, 4]);

        let expected: BTreeSet<Position> = [pos![4u8, 5u8], pos![5u8, 4u8]].into_iter().collect();
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
        let expected: BTreeSet<Position> = [pos![4u8, 3u8]].into_iter().collect();
        assert_eq!(moves, expected);
    }

    /// Threats follow the same filter: the WHITE-only capture must not make BLACK's
    /// walker attack anything, while it does let WHITE's walker attack.
    #[test]
    fn test_restricted_move_does_not_create_threats_for_other_players() {
        let mut game = load_game();
        insert(&mut game, [4, 4], "WHITE");
        insert(&mut game, [7, 7], "BLACK");

        assert!(game.attacked_by("WHITE").contains(&pos![5u8, 4u8]));
        assert!(!game.attacked_by("BLACK").contains(&pos![6u8, 7u8]));
        assert!(!game.attacked_by("BLACK").contains(&pos![8u8, 7u8]));
    }
}
