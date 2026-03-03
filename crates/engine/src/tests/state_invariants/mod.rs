#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load_side_effects_game() -> Game {
        parse_game_spec("./src/tests/side_effects/spec.json")
            .map(Game::from_spec)
            .expect("Failed to load side_effects test spec")
    }

    fn load_shogi() -> Game {
        parse_game_spec("./specs/shogi.json")
            .map(Game::from_spec)
            .expect("Failed to load shogi spec")
    }

    fn load_chess() -> Game {
        parse_game_spec("./specs/chess.json")
            .map(Game::from_spec)
            .expect("Failed to load chess spec")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    #[test]
    fn moving_phase_always_has_available_moves_map() {
        let mut game = load_side_effects_game();
        insert(&mut game, vec![1, 1], "DUMMY", "WHITE");

        game.transition(GameTransition::CalculateMoves {
            position: vec![1, 1],
        })
        .unwrap();

        assert!(matches!(game.state.phase, GamePhase::Moving { .. }));
        let moves = game
            .state
            .available_moves
            .as_ref()
            .expect("Moving phase should carry available_moves map");
        assert!(moves.is_empty(), "DUMMY should have no legal moves");
    }

    #[test]
    fn dropping_phase_always_has_available_moves_map() {
        let mut game = load_shogi();
        game.state.pieces.clear();

        let current_player = game.current_player();
        game.state
            .hand
            .entry(current_player.clone())
            .or_default()
            .insert("PAWN".to_string(), 1);

        game.transition(GameTransition::CalculateDrops {
            piece_code: "PAWN".to_string(),
        })
        .unwrap();

        assert!(matches!(game.state.phase, GamePhase::Dropping { .. }));
        assert!(
            game.state.available_moves.is_some(),
            "Dropping phase should carry available_moves map"
        );
    }

    #[test]
    fn invalid_selection_resets_to_idle_and_clears_available_moves() {
        let mut game = load_side_effects_game();
        insert(&mut game, vec![0, 0], "CASTLER", "WHITE");

        game.transition(GameTransition::CalculateMoves {
            position: vec![0, 0],
        })
        .unwrap();
        assert!(game.state.available_moves.is_some());

        let result = game.transition(GameTransition::ExecuteMove {
            position: vec![9, 9],
        });
        assert!(result.is_err(), "expected invalid move error");

        assert!(matches!(game.state.phase, GamePhase::Idle));
        assert!(
            game.state.available_moves.is_none(),
            "Idle state should clear available moves after invalid selection"
        );
    }

    #[test]
    fn execute_move_success_clears_available_moves_and_exits_moving_phase() {
        let mut game = load_chess();

        game.transition(GameTransition::CalculateMoves {
            position: vec![4, 1],
        })
        .unwrap();
        assert!(matches!(game.state.phase, GamePhase::Moving { .. }));
        assert!(game.state.available_moves.is_some());

        game.transition(GameTransition::ExecuteMove {
            position: vec![4, 3],
        })
        .unwrap();

        assert!(
            !matches!(game.state.phase, GamePhase::Moving { .. }),
            "ExecuteMove should leave moving phase"
        );
        assert!(
            game.state.available_moves.is_none(),
            "available_moves should be cleared after successful move execution"
        );
    }

    #[test]
    fn transforming_flow_keeps_available_moves_cleared_and_exits_transforming() {
        let mut game = load_chess();
        game.state.pieces.clear();
        insert(&mut game, vec![0, 6], "PAWN", "WHITE");
        insert(&mut game, vec![4, 0], "KING", "WHITE");
        insert(&mut game, vec![7, 7], "KING", "BLACK");

        game.transition(GameTransition::CalculateMoves {
            position: vec![0, 6],
        })
        .unwrap();
        game.transition(GameTransition::ExecuteMove {
            position: vec![0, 7],
        })
        .unwrap();

        assert!(
            matches!(game.state.phase, GamePhase::Transforming { .. }),
            "promotion move should enter transforming phase"
        );
        assert!(
            game.state.available_moves.is_none(),
            "transforming phase should not retain stale move maps"
        );

        game.transition(GameTransition::Transform {
            target: "QUEEN".to_string(),
        })
        .unwrap();

        assert!(
            !matches!(game.state.phase, GamePhase::Transforming { .. }),
            "Transform should exit transforming phase"
        );
        assert!(
            game.state.available_moves.is_none(),
            "available_moves should remain cleared after transform resolution"
        );
    }

    #[test]
    fn execute_drop_success_clears_available_moves_and_exits_dropping_phase() {
        let mut game = load_shogi();
        let current_player = game.current_player();
        game.state
            .hand
            .entry(current_player)
            .or_default()
            .insert("ROOK".to_string(), 1);

        game.transition(GameTransition::CalculateDrops {
            piece_code: "ROOK".to_string(),
        })
        .unwrap();
        assert!(matches!(game.state.phase, GamePhase::Dropping { .. }));
        let drop_target = game
            .state
            .available_moves
            .as_ref()
            .and_then(|moves| moves.keys().next().cloned())
            .expect("expected at least one legal rook drop square");

        game.transition(GameTransition::ExecuteDrop {
            position: drop_target,
        })
        .unwrap();

        assert!(
            !matches!(game.state.phase, GamePhase::Dropping { .. }),
            "ExecuteDrop should leave dropping phase"
        );
        assert!(
            game.state.available_moves.is_none(),
            "available_moves should be cleared after successful drop execution"
        );
    }
}
