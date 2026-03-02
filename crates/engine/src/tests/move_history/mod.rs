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
    // A single pawn move is recorded correctly
    // -------------------------------------------------------------------------

    #[test]
    fn test_move_recorded_after_pawn_advance() {
        let mut game = load_chess();

        // e2 → e4 (WHITE pawn)
        game.transition(GameTransition::CalculateMoves { position: vec![4, 1] }).unwrap();
        game.transition(GameTransition::ExecuteMove   { position: vec![4, 3] }).unwrap();

        let history = game.history();
        assert_eq!(history.len(), 1, "exactly one record after one move");

        let record = &history[0];
        assert_eq!(record.player,     "WHITE");
        assert_eq!(record.piece_code, "PAWN");
        assert_eq!(record.from,       vec![4u8, 1]);
        assert_eq!(record.to,         vec![4u8, 3]);
        assert_eq!(record.action,     "MOVE");
        assert!(record.promotion.is_none());
    }

    // -------------------------------------------------------------------------
    // A capture is recorded with action="CAPTURE"
    // -------------------------------------------------------------------------

    #[test]
    fn test_capture_recorded_with_correct_action() {
        let mut game = load_chess();

        // e2→e4
        game.transition(GameTransition::CalculateMoves { position: vec![4, 1] }).unwrap();
        game.transition(GameTransition::ExecuteMove   { position: vec![4, 3] }).unwrap();
        // d7→d5
        game.transition(GameTransition::CalculateMoves { position: vec![3, 6] }).unwrap();
        game.transition(GameTransition::ExecuteMove   { position: vec![3, 4] }).unwrap();
        // e4 captures d5
        game.transition(GameTransition::CalculateMoves { position: vec![4, 3] }).unwrap();
        game.transition(GameTransition::ExecuteMove   { position: vec![3, 4] }).unwrap();

        let history = game.history();
        assert_eq!(history.len(), 3, "three records after three half-moves");

        let capture = &history[2];
        assert_eq!(capture.player,     "WHITE");
        assert_eq!(capture.piece_code, "PAWN");
        assert_eq!(capture.from,       vec![4u8, 3]);
        assert_eq!(capture.to,         vec![3u8, 4]);
        assert_eq!(capture.action,     "CAPTURE");
        assert!(capture.promotion.is_none());
    }

    // -------------------------------------------------------------------------
    // History accumulates across many moves
    // -------------------------------------------------------------------------

    #[test]
    fn test_history_accumulates() {
        let mut game = load_chess();

        let moves = [
            (vec![4u8, 1u8], vec![4u8, 3u8]), // e2→e4
            (vec![4u8, 6u8], vec![4u8, 4u8]), // e7→e5
            (vec![3u8, 1u8], vec![3u8, 3u8]), // d2→d4
            (vec![3u8, 6u8], vec![3u8, 4u8]), // d7→d5
        ];

        for (from, to) in &moves {
            game.transition(GameTransition::CalculateMoves { position: from.clone() }).unwrap();
            game.transition(GameTransition::ExecuteMove   { position: to.clone()   }).unwrap();
        }

        assert_eq!(game.history().len(), moves.len());
    }

    // -------------------------------------------------------------------------
    // Serialization round-trip preserves history
    // -------------------------------------------------------------------------

    #[test]
    fn test_history_survives_save_restore() {
        let mut game = load_chess();

        game.transition(GameTransition::CalculateMoves { position: vec![4, 1] }).unwrap();
        game.transition(GameTransition::ExecuteMove   { position: vec![4, 3] }).unwrap();

        let json = game.save_state().unwrap();
        game.restore_state(&json).unwrap();

        let history = game.history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].from, vec![4u8, 1]);
        assert_eq!(history[0].to,   vec![4u8, 3]);
    }

    // -------------------------------------------------------------------------
    // Promotion fills in the `promotion` field on the TRANSFORM record
    // -------------------------------------------------------------------------

    #[test]
    fn test_transform_record_has_promotion() {
        let mut game = load_chess();

        // Fastest path to promotion: clear all pieces between pawn and the last rank,
        // then march the pawn up directly by manipulating the board.
        // Strategy: place WHITE PAWN at [0,6] so it can reach [0,7] in one step.
        // Clear board: remove existing pieces that could interfere.
        game.state.pieces.retain(|_, p| p.player == "WHITE" || p.player == "BLACK");

        // Remove everything on the board and place a lone WHITE PAWN at [0,6]
        // and the BLACK KING somewhere safe (game-over detection needs pieces).
        use crate::logic::Piece;
        game.state.pieces.clear();
        game.state.pieces.insert(vec![0, 6], Piece::new("PAWN".into(), "WHITE".into()));
        game.state.pieces.insert(vec![7, 7], Piece::new("KING".into(), "WHITE".into()));
        game.state.pieces.insert(vec![7, 0], Piece::new("KING".into(), "BLACK".into()));

        // WHITE moves: pawn [0,6] → [0,7] (promotion rank)
        game.transition(GameTransition::CalculateMoves { position: vec![0, 6] }).unwrap();
        game.transition(GameTransition::ExecuteMove   { position: vec![0, 7] }).unwrap();

        // Should now be in Transforming phase.
        use crate::logic::GamePhase;
        assert!(
            matches!(game.state.phase, GamePhase::Transforming { .. }),
            "expected Transforming phase after pawn reaches last rank"
        );

        // Resolve promotion.
        game.transition(GameTransition::Transform { target: "QUEEN".into() }).unwrap();

        let history = game.history();
        assert_eq!(history.len(), 1, "one history record for the pawn move");
        assert_eq!(history[0].action, "TRANSFORM");
        assert_eq!(history[0].promotion, Some("QUEEN".into()));
    }
}
