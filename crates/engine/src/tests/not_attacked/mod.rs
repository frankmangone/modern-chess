#[cfg(test)]
mod tests {
    use crate::logic::{Game, Piece};
    use crate::shared::Position;
    use crate::specs::parse_game_spec;
    use crate::tests::support::{pick, targets};

    fn load_game() -> Game {
        parse_game_spec("./src/tests/not_attacked/spec.json")
            .map(Game::from_spec)
            .expect("Failed to load not_attacked test spec")
    }

    fn insert(game: &mut Game, pos: Position, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    fn has_no_legal_moves(game: &Game) -> bool {
        targets(game).is_empty()
    }

    // -------------------------------------------------------------------------
    // NOT ATTACKED at TARGET (NOT_ATTACKED before tests)
    // -------------------------------------------------------------------------

    /// GUARDED_MOVER (WHITE) at [4,4] with no opponents — target [5,4] is unattacked.
    /// The move should be available.
    #[test]
    fn test_not_attacked_allows_move_to_safe_square() {
        let mut game = load_game();
        insert(&mut game, pos![4, 4], "GUARDED_MOVER", "WHITE");

        pick(&mut game, pos![4, 4]).unwrap();
        assert!(
            targets(&game).contains(&pos![5u8, 4u8]),
            "Move to [5,4] should be available: square is not attacked"
        );
    }

    /// GUARDED_MOVER (WHITE) at [4,4], BLACK ROOK_THREAT at [5,9].
    /// The rook's [0,-1] direction (BLACK-transformed from spec [0,1]) attacks column x=5
    /// downward, including [5,4]. The NOT ATTACKED test must block the move.
    #[test]
    fn test_not_attacked_blocks_move_to_attacked_square() {
        let mut game = load_game();
        insert(&mut game, pos![4, 4], "GUARDED_MOVER", "WHITE");
        insert(&mut game, pos![5, 9], "ROOK_THREAT", "BLACK");

        pick(&mut game, pos![4, 4]).unwrap();
        assert!(
            has_no_legal_moves(&game),
            "Move to [5,4] should be blocked: BLACK rook at [5,9] attacks [5,4]"
        );
    }

    // -------------------------------------------------------------------------
    // NOT ATTACKED over the PATH to TARGET (PATH_NOT_ATTACKED before tests)
    // -------------------------------------------------------------------------

    /// PATH_GUARDED_MOVER (WHITE) at [4,4] with no opponents — path [4,5] and target [4,6]
    /// are both unattacked. The move should be available.
    #[test]
    fn test_path_not_attacked_allows_jump_when_path_clear() {
        let mut game = load_game();
        insert(&mut game, pos![4, 4], "PATH_GUARDED_MOVER", "WHITE");

        pick(&mut game, pos![4, 4]).unwrap();
        assert!(
            targets(&game).contains(&pos![4u8, 6u8]),
            "Move to [4,6] should be available: path through [4,5] is not attacked"
        );
    }

    /// PATH_GUARDED_MOVER (WHITE) at [4,4], BLACK ROOK_THREAT at [9,5].
    /// The rook's [-1,0] direction (BLACK-transformed from spec [1,0]) attacks row y=5
    /// leftward, including [4,5]. The path test checks [4,5] — attacked → blocks.
    #[test]
    fn test_path_not_attacked_blocks_jump_when_path_attacked() {
        let mut game = load_game();
        insert(&mut game, pos![4, 4], "PATH_GUARDED_MOVER", "WHITE");
        insert(&mut game, pos![9, 5], "ROOK_THREAT", "BLACK");

        pick(&mut game, pos![4, 4]).unwrap();
        assert!(
            has_no_legal_moves(&game),
            "Move to [4,6] should be blocked: BLACK rook at [9,5] attacks [4,5] on the path"
        );
    }
}
