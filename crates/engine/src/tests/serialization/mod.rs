#[cfg(test)]
mod tests {
    use crate::logic::{Game, GameTransition};
    use crate::specs::parse_game_spec;

    fn load_chess() -> Game {
        parse_game_spec("./specs/chess.json")
            .map(Game::from_spec)
            .expect("Failed to load chess spec")
    }

    // -------------------------------------------------------------------------
    // Round-trip: state serializes and restores faithfully
    // -------------------------------------------------------------------------

    #[test]
    fn test_save_and_restore_state_round_trips() {
        let mut game = load_chess();

        // Make a few moves so the state is non-trivial.
        game.transition(GameTransition::CalculateMoves { position: vec![4, 1] }).unwrap();
        game.transition(GameTransition::ExecuteMove   { position: vec![4, 3] }).unwrap(); // e2→e4
        game.transition(GameTransition::CalculateMoves { position: vec![4, 6] }).unwrap();
        game.transition(GameTransition::ExecuteMove   { position: vec![4, 4] }).unwrap(); // e7→e5

        // Capture the current state before saving.
        let pieces_before  = game.state.pieces.clone();
        let turn_before    = game.state.current_turn;
        let phase_before   = game.state.phase.clone();

        // Serialize.
        let json = game.save_state().expect("save_state should not fail");
        assert!(!json.is_empty());

        // Wipe the state to prove restore actually does something.
        game.state.pieces.clear();
        game.state.current_turn = 99;

        // Restore.
        game.restore_state(&json).expect("restore_state should not fail");

        // Structural equality checks.
        assert_eq!(game.state.pieces,        pieces_before,  "pieces mismatch after restore");
        assert_eq!(game.state.current_turn,  turn_before,    "current_turn mismatch after restore");
        assert_eq!(game.state.phase,         phase_before,   "phase mismatch after restore");
        assert!(game.state.available_moves.is_none(),        "available_moves should be None after restore");
    }

    // -------------------------------------------------------------------------
    // Engine stays functional after restore
    // -------------------------------------------------------------------------

    #[test]
    fn test_engine_functional_after_restore() {
        let mut game = load_chess();

        // e4
        game.transition(GameTransition::CalculateMoves { position: vec![4, 1] }).unwrap();
        game.transition(GameTransition::ExecuteMove   { position: vec![4, 3] }).unwrap();

        let json = game.save_state().unwrap();
        game.restore_state(&json).unwrap();

        // It should now be BLACK's turn and CalculateMoves should work normally.
        let result = game.transition(GameTransition::CalculateMoves { position: vec![4, 6] });
        assert!(result.is_ok(), "CalculateMoves should succeed after restore");
        assert!(
            game.state.available_moves.is_some(),
            "available_moves should be populated after CalculateMoves post-restore"
        );
    }

    // -------------------------------------------------------------------------
    // JSON format sanity: pieces appear as string-keyed entries
    // -------------------------------------------------------------------------

    #[test]
    fn test_serialized_json_uses_string_keys_for_pieces() {
        let game = load_chess();
        let json = game.save_state().unwrap();

        // Position keys are comma-separated strings like "4,1", not arrays.
        // Check that the JSON contains at least one such key pattern.
        assert!(
            json.contains("\"4,1\"") || json.contains("\"0,0\""),
            "JSON should use comma-separated string keys for piece positions, got: {}",
            &json[..200.min(json.len())]
        );
    }
}
