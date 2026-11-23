#[cfg(test)]
mod tests {
    use crate::logic::{Answer, Game};
    use crate::specs::{parse_game_spec, GameSpecError};
    use crate::tests::support::{chosen, go, phase_is_choosing, pick, promote, promotion_options};

    #[test]
    fn test_transformation() -> Result<(), GameSpecError> {
        // Load chess specification
        let game_spec = parse_game_spec("./src/tests/transformation/spec.json")?;
        let mut game = Game::from_spec(game_spec);

        assert!(pick(&mut game, pos![1, 0]).is_ok());
        assert!(go(&mut game, pos![1, 1]).is_ok());

        // The piece stands on [1,1] waiting for its promotion.
        assert_eq!(chosen(&game).last(), Some(&Answer::Place(pos![1, 1])));
        assert_eq!(
            game.shown_pieces()
                .get(&pos![1, 1])
                .map(|piece| piece.code.as_str()),
            Some("PAWN")
        );
        assert_eq!(
            promotion_options(&game),
            vec!["QUEEN", "ROOK", "BISHOP", "KNIGHT"]
        );

        assert!(promote(&mut game, "QUEEN").is_ok());
        assert_eq!(game.state.pieces.get(&pos![1, 1]).unwrap().code, "QUEEN");

        assert!(phase_is_choosing(&game));

        Ok(())
    }
}
