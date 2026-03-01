#[cfg(test)]
mod tests {
    use crate::logic::{Game, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load_game() -> Game {
        parse_game_spec("./src/tests/not_attacked/spec.json")
            .map(Game::from_spec)
            .expect("Failed to load not_attacked test spec")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, code: &str, player: &str) {
        game.state.pieces.insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    // -------------------------------------------------------------------------
    // NOT_ATTACKED
    // -------------------------------------------------------------------------

    /// GUARDED_MOVER (WHITE) at [4,4] with no opponents — target [5,4] is unattacked.
    /// The move should be available.
    #[test]
    fn test_not_attacked_allows_move_to_safe_square() {
        let mut game = load_game();
        insert(&mut game, vec![4, 4], "GUARDED_MOVER", "WHITE");

        game.transition(GameTransition::CalculateMoves { position: vec![4, 4] }).unwrap();
        assert!(
            game.state.available_moves.as_ref().unwrap().contains_key(&vec![5u8, 4u8]),
            "Move to [5,4] should be available: square is not attacked"
        );
    }

    /// GUARDED_MOVER (WHITE) at [4,4], BLACK ROOK_THREAT at [5,9].
    /// The rook's [0,-1] direction (BLACK-transformed from spec [0,1]) attacks column x=5
    /// downward, including [5,4]. NOT_ATTACKED must block the move.
    #[test]
    fn test_not_attacked_blocks_move_to_attacked_square() {
        let mut game = load_game();
        insert(&mut game, vec![4, 4], "GUARDED_MOVER", "WHITE");
        insert(&mut game, vec![5, 9], "ROOK_THREAT", "BLACK");

        game.transition(GameTransition::CalculateMoves { position: vec![4, 4] }).unwrap();
        assert!(
            game.state.available_moves.is_none(),
            "Move to [5,4] should be blocked: BLACK rook at [5,9] attacks [5,4]"
        );
    }

    // -------------------------------------------------------------------------
    // PATH_NOT_ATTACKED
    // -------------------------------------------------------------------------

    /// PATH_GUARDED_MOVER (WHITE) at [4,4] with no opponents — path [4,5] and target [4,6]
    /// are both unattacked. The move should be available.
    #[test]
    fn test_path_not_attacked_allows_jump_when_path_clear() {
        let mut game = load_game();
        insert(&mut game, vec![4, 4], "PATH_GUARDED_MOVER", "WHITE");

        game.transition(GameTransition::CalculateMoves { position: vec![4, 4] }).unwrap();
        assert!(
            game.state.available_moves.as_ref().unwrap().contains_key(&vec![4u8, 6u8]),
            "Move to [4,6] should be available: path through [4,5] is not attacked"
        );
    }

    /// PATH_GUARDED_MOVER (WHITE) at [4,4], BLACK ROOK_THREAT at [9,5].
    /// The rook's [-1,0] direction (BLACK-transformed from spec [1,0]) attacks row y=5
    /// leftward, including [4,5]. PATH_NOT_ATTACKED checks [4,5] (i=1) — attacked → blocks.
    #[test]
    fn test_path_not_attacked_blocks_jump_when_path_attacked() {
        let mut game = load_game();
        insert(&mut game, vec![4, 4], "PATH_GUARDED_MOVER", "WHITE");
        insert(&mut game, vec![9, 5], "ROOK_THREAT", "BLACK");

        game.transition(GameTransition::CalculateMoves { position: vec![4, 4] }).unwrap();
        assert!(
            game.state.available_moves.is_none(),
            "Move to [4,6] should be blocked: BLACK rook at [9,5] attacks [4,5] on the path"
        );
    }
}
