
#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition};
    use crate::specs::{parse_game_spec, GameSpecError};

    #[test]
    fn test_transformation() -> Result<(), GameSpecError> {
        // Load chess specification
        let game_spec = parse_game_spec("./src/tests/transformation/spec.json")?;
        let mut game = Game::from_spec(game_spec);

        assert!(game.transition(GameTransition::CalculateMoves{ position: vec![1, 0] }).is_ok());
        assert!(game.transition(GameTransition::ExecuteMove{ position: vec![1, 1] }).is_ok());

        match &game.state.phase {
            GamePhase::Transforming { position, options } => {
                assert_eq!(position, &vec![1, 1]);
                assert_eq!(options, &vec!["QUEEN", "ROOK", "BISHOP", "KNIGHT"]);
            }
            _ => {
                assert!(false);
            }
        }

        assert!(game.transition(GameTransition::Transform{ target: "QUEEN".to_string() }).is_ok());
        assert_eq!(game.state.pieces.get(&vec![1, 1]).unwrap().code, "QUEEN");

        assert!(game.state.phase == GamePhase::Idle);
        
        Ok(())
    }
}