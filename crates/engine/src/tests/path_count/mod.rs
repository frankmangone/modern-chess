#[cfg(test)]
mod tests {
    use crate::logic::{Game, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load_game() -> Game {
        parse_game_spec("./src/tests/path_count/spec.json")
            .map(Game::from_spec)
            .expect("Failed to load path_count test spec")
    }

    fn insert(game: &mut Game, pos: [u8; 2], code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos.to_vec(), Piece::new(code.to_string(), player.to_string()));
    }

    /// Whether the PROBE at `from` is offered its one-square step, for the current player.
    fn probe_can_step(game: &mut Game, from: [u8; 2], to: [u8; 2]) -> bool {
        game.transition(GameTransition::CalculateMoves {
            position: from.to_vec(),
        })
        .expect("selecting the piece should succeed");
        game.state
            .available_moves
            .as_ref()
            .map_or(false, |m| m.contains_key(&to.to_vec()))
    }

    /// The PROBE's step is guarded by "nothing strictly between me and the square 4 to my
    /// right". With a clear row it is offered.
    #[test]
    fn test_clear_path_to_the_position_allows_the_move() {
        let mut game = load_game();
        insert(&mut game, [2, 5], "PROBE", "WHITE");

        assert!(probe_can_step(&mut game, [2, 5], [3, 5]));
    }

    /// A piece on [5,5] sits between [2,5] and [6,5] but is *not* on the target of the step
    /// ([3,5]); only the `position` endpoint makes the condition see it.
    #[test]
    fn test_piece_between_source_and_the_position_blocks_the_move() {
        let mut game = load_game();
        insert(&mut game, [2, 5], "PROBE", "WHITE");
        insert(&mut game, [5, 5], "BLOCK", "BLACK");

        assert!(!probe_can_step(&mut game, [2, 5], [3, 5]));
    }

    /// The endpoint itself is not "between": a piece standing exactly on [6,5] (the square
    /// 4 to the right) does not block.
    #[test]
    fn test_piece_on_the_position_itself_does_not_block() {
        let mut game = load_game();
        insert(&mut game, [2, 5], "PROBE", "WHITE");
        insert(&mut game, [6, 5], "BLOCK", "BLACK");

        assert!(probe_can_step(&mut game, [2, 5], [3, 5]));
    }

    /// `position` is written in the neutral frame and rotated per player, like every other
    /// relative offset: for BLACK, "4 to the right" is 4 to the left.
    #[test]
    fn test_position_is_rotated_for_the_other_player() {
        let mut game = load_game();
        insert(&mut game, [0, 0], "BLOCK", "WHITE"); // keeps WHITE present
        insert(&mut game, [7, 5], "PROBE", "BLACK");
        insert(&mut game, [4, 5], "BLOCK", "WHITE"); // 3 to the left of the probe
        game.state.current_turn = 1; // BLACK to move

        assert!(!probe_can_step(&mut game, [7, 5], [6, 5]));
    }
}
