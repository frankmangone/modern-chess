#[cfg(test)]
mod tests {
    use super::super::board::BoardSpec;
    use super::super::draw_conditions::DrawConditionsSpec;
    use super::super::game::{GameSpec, GameSpecError};
    use super::super::liberty_capture::LibertyCaptureSpec;
    use super::super::player::PlayerSpec;
    use super::super::turns::TurnSpec;
    use super::super::win_condition::WinConditionSpec;

    // Define constants we'll use throughout the tests.
    const PLAYER_1_NAME: &str = "Player1";
    const PLAYER_2_NAME: &str = "Player2";
    const UNKNOWN_PLAYER_NAME: &str = "UnknownPlayer";

    /// Helper function to create a valid game spec to test with.
    fn create_valid_game_spec() -> GameSpec {
        GameSpec {
            name: "Test Game".to_string(),
            pieces: vec![],
            board: BoardSpec::default(),
            players: vec![
                PlayerSpec::from_name(PLAYER_1_NAME),
                PlayerSpec::from_name(PLAYER_2_NAME),
            ],
            turns: TurnSpec::from_order(vec![PLAYER_1_NAME, PLAYER_2_NAME]),
            conditions: vec![],
            leader: vec![],
            draw_conditions: DrawConditionsSpec::default(),
            stalemate_loses: false,
            hand_enabled: false,
            no_opponent_check: false,
            jump_enabled: false,
            mandatory_capture: false,
            allow_pass: false,
            forbid_repeated_positions: false,
            liberty_capture: None,
            win_conditions: vec![],
        }
    }

    #[test]
    fn test_valid_game_spec() {
        let game_spec = create_valid_game_spec();
        assert!(game_spec.validate_specs().is_ok());
    }

    #[test]
    fn test_duplicate_player_names() {
        let mut game_spec = create_valid_game_spec();
        game_spec.players.push(PlayerSpec::from_name(PLAYER_1_NAME));

        match game_spec.validate_specs() {
            Err(GameSpecError::DuplicatePlayerName(name)) => assert_eq!(name, PLAYER_1_NAME),
            _ => panic!("Expected `DuplicatePlayerName` error"),
        }
    }

    #[test]
    fn test_unknown_player_in_turn_order() {
        let mut game_spec = create_valid_game_spec();
        game_spec.turns.order.push(UNKNOWN_PLAYER_NAME.to_string());

        match game_spec.validate_specs() {
            Err(GameSpecError::UnknownPlayerInTurnOrder(name)) => {
                assert_eq!(name, UNKNOWN_PLAYER_NAME)
            }
            _ => panic!("Expected `UnknownPlayerInTurnOrder` error"),
        }
    }

    #[test]
    fn test_valid_turn_order() {
        let mut game_spec = create_valid_game_spec();
        game_spec.turns.order = vec![PLAYER_2_NAME.to_string(), PLAYER_1_NAME.to_string()];
        assert!(game_spec.validate_specs().is_ok());
    }

    #[test]
    fn test_empty_player_list() {
        let mut game_spec = create_valid_game_spec();
        game_spec.players.clear();
        game_spec.turns.order.clear();
        assert!(game_spec.validate_specs().is_ok());
    }

    #[test]
    fn test_single_player() {
        let mut game_spec = create_valid_game_spec();
        game_spec.players = vec![PlayerSpec::from_name(PLAYER_1_NAME)];
        game_spec.turns.order = vec![PLAYER_1_NAME.to_string()];
        assert!(game_spec.validate_specs().is_ok());
    }

    #[test]
    fn test_liberty_capture_must_name_a_known_piece() {
        let mut game_spec = create_valid_game_spec();
        game_spec.liberty_capture = Some(LibertyCaptureSpec {
            pieces: vec!["STONE".to_string()],
            allow_suicide: false,
        });
        assert!(matches!(
            game_spec.validate_specs(),
            Err(GameSpecError::UnknownPieceInRule(code)) if code == "STONE"
        ));
    }

    #[test]
    fn test_area_score_must_name_a_known_piece() {
        let mut game_spec = create_valid_game_spec();
        game_spec.win_conditions = vec![serde_json::from_str::<WinConditionSpec>(
            r#"{ "type": "AREA_SCORE", "pieces": ["DISC"] }"#,
        )
        .unwrap()];
        assert!(matches!(
            game_spec.validate_specs(),
            Err(GameSpecError::UnknownPieceInRule(code)) if code == "DISC"
        ));
    }

    #[test]
    fn test_area_score_without_pieces_counts_everything_and_validates() {
        let mut game_spec = create_valid_game_spec();
        game_spec.win_conditions =
            vec![serde_json::from_str::<WinConditionSpec>(r#"{ "type": "AREA_SCORE" }"#).unwrap()];
        assert!(game_spec.validate_specs().is_ok());
    }
}
