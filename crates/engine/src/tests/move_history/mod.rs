#[cfg(test)]
mod tests {
    use crate::logic::{Answer, Game, MoveRecord};
    use crate::specs::parse_game_spec;
    use crate::tests::support::{go, pick, promote, promotion_options};

    /// The promotion a record's answers chose, if any.
    fn promotion(record: &MoveRecord) -> Option<&str> {
        record.chosen.iter().find_map(|answer| match answer {
            Answer::Code(code) => Some(code.as_str()),
            _ => None,
        })
    }

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
        pick(&mut game, pos![4, 1]).unwrap();
        go(&mut game, pos![4, 3]).unwrap();

        let history = game.history();
        assert_eq!(history.len(), 1, "exactly one record after one move");

        let record = &history[0];
        assert_eq!(record.player, "WHITE");
        assert_eq!(record.piece_code, "PAWN");
        assert_eq!(record.from, pos![4u8, 1]);
        assert_eq!(record.to, pos![4u8, 3]);
        assert_eq!(record.action, "MOVE");
        assert!(!record.captures, "a plain move is no capture");
        assert!(promotion(record).is_none());
    }

    // -------------------------------------------------------------------------
    // A capture is recorded with the capture mark
    // -------------------------------------------------------------------------

    #[test]
    fn test_capture_recorded_with_correct_action() {
        let mut game = load_chess();

        // e2→e4
        pick(&mut game, pos![4, 1]).unwrap();
        go(&mut game, pos![4, 3]).unwrap();
        // d7→d5
        pick(&mut game, pos![3, 6]).unwrap();
        go(&mut game, pos![3, 4]).unwrap();
        // e4 captures d5
        pick(&mut game, pos![4, 3]).unwrap();
        go(&mut game, pos![3, 4]).unwrap();

        let history = game.history();
        assert_eq!(history.len(), 3, "three records after three half-moves");

        let capture = &history[2];
        assert_eq!(capture.player, "WHITE");
        assert_eq!(capture.piece_code, "PAWN");
        assert_eq!(capture.from, pos![4u8, 3]);
        assert_eq!(capture.to, pos![3u8, 4]);
        assert!(capture.captures, "the record carries the capture mark");
        assert!(promotion(capture).is_none());
    }

    // -------------------------------------------------------------------------
    // History accumulates across many moves
    // -------------------------------------------------------------------------

    #[test]
    fn test_history_accumulates() {
        let mut game = load_chess();

        let moves = [
            (pos![4u8, 1u8], pos![4u8, 3u8]), // e2→e4
            (pos![4u8, 6u8], pos![4u8, 4u8]), // e7→e5
            (pos![3u8, 1u8], pos![3u8, 3u8]), // d2→d4
            (pos![3u8, 6u8], pos![3u8, 4u8]), // d7→d5
        ];

        for (from, to) in &moves {
            pick(&mut game, *from).unwrap();
            go(&mut game, *to).unwrap();
        }

        assert_eq!(game.history().len(), moves.len());
    }

    // -------------------------------------------------------------------------
    // Serialization round-trip preserves history
    // -------------------------------------------------------------------------

    #[test]
    fn test_history_survives_save_restore() {
        let mut game = load_chess();

        pick(&mut game, pos![4, 1]).unwrap();
        go(&mut game, pos![4, 3]).unwrap();

        let json = game.save_state().unwrap();
        game.restore_state(&json).unwrap();

        let history = game.history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].from, pos![4u8, 1]);
        assert_eq!(history[0].to, pos![4u8, 3]);
    }

    // -------------------------------------------------------------------------
    // A promotion's code is among the answers of the move's record
    // -------------------------------------------------------------------------

    #[test]
    fn test_transform_record_has_promotion() {
        let mut game = load_chess();

        // Fastest path to promotion: clear all pieces between pawn and the last rank,
        // then march the pawn up directly by manipulating the board.
        // Strategy: place WHITE PAWN at [0,6] so it can reach [0,7] in one step.
        // Clear board: remove existing pieces that could interfere.
        game.state
            .pieces
            .retain(|_, p| p.player == "WHITE" || p.player == "BLACK");

        // Remove everything on the board and place a lone WHITE PAWN at [0,6]
        // and the BLACK KING somewhere safe (game-over detection needs pieces).
        use crate::logic::Piece;
        game.state.pieces.clear();
        game.state
            .pieces
            .insert(pos![0, 6], Piece::new("PAWN".into(), "WHITE".into()));
        game.state
            .pieces
            .insert(pos![7, 7], Piece::new("KING".into(), "WHITE".into()));
        game.state
            .pieces
            .insert(pos![7, 0], Piece::new("KING".into(), "BLACK".into()));

        // WHITE moves: pawn [0,6] → [0,7] (promotion rank)
        pick(&mut game, pos![0, 6]).unwrap();
        go(&mut game, pos![0, 7]).unwrap();

        // Should now be asking for the promotion.
        assert!(
            !promotion_options(&game).is_empty(),
            "expected a promotion to be asked after pawn reaches last rank"
        );

        // Resolve promotion.
        promote(&mut game, "QUEEN").unwrap();

        let history = game.history();
        assert_eq!(history.len(), 1, "one history record for the pawn move");
        assert_eq!(
            history[0].chosen,
            vec![
                Answer::Place(pos![0, 6]),
                Answer::Place(pos![0, 7]),
                Answer::Code("QUEEN".into())
            ],
            "the record is the promoting move, with the chosen promotion"
        );
        assert_eq!(promotion(&history[0]), Some("QUEEN"));
    }
}
