//! A chess state saved by the engine before boards became graphs. Stored matches keep their
//! state as JSON for good, so this one must keep loading, counting and saving the same way.
//!
//! The fixture is 1.e4 e5 2.Nf3 Nc6 3.Ng1 Nb8 4.Nf3, with Black holding the f8 bishop.
#[cfg(test)]
mod tests {
    use crate::shared::Position;
    use serde_json::Value;

    use crate::logic::{Game, GamePhase, GameTransition};
    use crate::specs::parse_game_spec;

    const FIXTURE: &str = include_str!("chess_midgame.json");

    fn restored() -> Game {
        let mut game = parse_game_spec("./specs/chess.json")
            .map(Game::from_spec)
            .expect("chess loads");
        game.restore_state(FIXTURE).expect("the stored state loads");
        game
    }

    fn play(game: &mut Game, from: Position, to: Position) {
        game.transition(GameTransition::CalculateMoves { position: from })
            .unwrap();
        game.transition(GameTransition::ExecuteMove { position: to })
            .unwrap();
    }

    #[test]
    fn a_state_saved_before_the_graph_loads_and_saves_unchanged() {
        let game = restored();
        let saved_again: Value = serde_json::from_str(&game.save_state().unwrap()).unwrap();
        let mut fixture: Value = serde_json::from_str(FIXTURE).unwrap();
        // The fixture predates saving the turn order; it is filled in from the spec on restore.
        fixture["turn_order"] = serde_json::json!(["WHITE", "BLACK"]);
        assert_eq!(saved_again, fixture);
    }

    #[test]
    fn a_restored_state_keeps_its_repetition_keys() {
        let game = restored();
        let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
        let last = fixture["position_hashes"]
            .as_array()
            .and_then(|hashes| hashes.last())
            .and_then(Value::as_str)
            .unwrap();
        assert_eq!(game.position_key(), last);
    }

    #[test]
    fn a_restored_state_plays_on_and_counts_the_repetition() {
        let mut game = restored();
        play(&mut game, pos![1, 7], pos![2, 5]); // 4...Nc6
        play(&mut game, pos![5, 2], pos![6, 0]); // 5.Ng1
        play(&mut game, pos![2, 5], pos![1, 7]); // 5...Nb8
        play(&mut game, pos![6, 0], pos![5, 2]); // 6.Nf3
        assert_eq!(game.state.phase, GamePhase::Idle);
        play(&mut game, pos![1, 7], pos![2, 5]); // 6...Nc6, the third time after 2...Nc6
        assert_eq!(game.state.phase, GamePhase::GameOver { winner: None });
    }
}
