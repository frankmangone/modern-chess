//! The engine's one protocol: the player answers the choices of their turn with `Choose`, and
//! takes the answers back with `Cancel`. Answers filter the ways worked out when the turn
//! started, and once they match exactly one complete way, it is played.
#[cfg(test)]
mod tests {
    use crate::logic::{
        ActionOption, Answer, ChoiceKind, Game, GameError, GamePhase, GameTransition, Piece,
        PlaceOption,
    };
    use crate::shared::Position;
    use crate::specs::{parse_game_spec, parse_game_spec_str};
    use crate::tests::support::chosen;

    const WAYS: &str = "./src/tests/ways/spec.json";

    /// A turn's spec with `jump_enabled`: a man jumps an enemy straight ahead (and may go on
    /// jumping), or one to its right, which ends the turn. It can also step ahead.
    const JUMPS: &str = r#"{
      "name": "JUMPS",
      "board": {"dimensions": [8, 8]},
      "players": [
        {"name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": []},
        {"name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": []}
      ],
      "turns": {"order": ["WHITE", "BLACK"]},
      "jump_enabled": true,
      "pieces": [
        {
          "code": "MAN",
          "name": "man",
          "moves": [
            {
              "id": 0,
              "step": [0, 2],
              "do": [
                {"test": "NOT", "of": {"test": "COUNT", "in": {"region": "AT", "at": {"place": "TARGET"}}}},
                {"test": "COUNT", "in": {"region": "AT", "at": {"place": "START", "offset": [0, 1]}}, "where": {"owner": "ENEMY"}},
                {"effect": "REMOVE", "at": {"place": "START", "offset": [0, 1]}},
                {"effect": "MOVE", "from": {"place": "START"}, "to": {"place": "TARGET"}}
              ]
            },
            {
              "id": 1,
              "step": [2, 0],
              "do": [
                {"test": "NOT", "of": {"test": "COUNT", "in": {"region": "AT", "at": {"place": "TARGET"}}}},
                {"test": "COUNT", "in": {"region": "AT", "at": {"place": "START", "offset": [1, 0]}}, "where": {"owner": "ENEMY"}},
                {"effect": "REMOVE", "at": {"place": "START", "offset": [1, 0]}},
                {"effect": "MOVE", "from": {"place": "START"}, "to": {"place": "TARGET"}},
                {"effect": "END_TURN"}
              ]
            },
            {
              "id": 2,
              "step": [0, 1],
              "do": [
                {"test": "NOT", "of": {"test": "COUNT", "in": {"region": "AT", "at": {"place": "TARGET"}}}},
                {"effect": "MOVE", "from": {"place": "START"}, "to": {"place": "TARGET"}}
              ]
            }
          ]
        }
      ],
      "actions": [
        {"name": "MOVE", "do": [{"choose": "PIECE", "where": {"owner": "ALLY"}}, {"choose": "MOVE"}]}
      ]
    }"#;

    fn load(path: &str) -> Game {
        parse_game_spec(path)
            .map(Game::from_spec)
            .unwrap_or_else(|error| panic!("{path} is read: {error:?}"))
    }

    fn chess() -> Game {
        load("./specs/chess.json")
    }

    /// A game of `spec` with only the pieces given, the ways worked out for them.
    fn with_pieces(mut game: Game, pieces: &[(Position, &str, &str)]) -> Game {
        game.state.pieces.clear();
        for (at, code, player) in pieces {
            game.state
                .pieces
                .insert(*at, Piece::new(code.to_string(), player.to_string()));
        }
        game.work_out_ways();
        game
    }

    fn choose(game: &mut Game, answer: Answer) -> Result<(), GameError> {
        game.transition(GameTransition::Choose(answer))
    }

    fn place(at: Position) -> Answer {
        Answer::Place(at)
    }

    fn code(code: &str) -> Answer {
        Answer::Code(code.to_string())
    }

    fn action(name: &str) -> Answer {
        Answer::Action(name.to_string())
    }

    fn cancel(game: &mut Game) -> Result<(), GameError> {
        game.transition(GameTransition::Cancel)
    }

    fn code_at(game: &Game, at: Position) -> Option<String> {
        game.state
            .pieces
            .get(&at)
            .map(|piece| piece.code.to_string())
    }

    fn state_json(game: &Game) -> String {
        game.save_state().expect("a state is written")
    }

    /// Checkers with a man that can jump twice in a row: RED to move.
    fn double_jump() -> Game {
        with_pieces(
            load("./specs/checkers.json"),
            &[
                (pos![3, 1], "MAN", "RED"),
                (pos![0, 1], "MAN", "RED"),
                (pos![4, 2], "MAN", "BLACK"),
                (pos![4, 4], "MAN", "BLACK"),
                (pos![0, 7], "MAN", "BLACK"),
            ],
        )
    }

    /// Chess with WHITE's pawn one step from promoting.
    fn about_to_promote() -> Game {
        with_pieces(
            chess(),
            &[
                (pos![4, 0], "KING", "WHITE"),
                (pos![4, 7], "KING", "BLACK"),
                (pos![0, 6], "PAWN", "WHITE"),
            ],
        )
    }

    #[test]
    fn choosing_a_piece_then_a_place_plays_the_move() {
        let mut game = chess();

        choose(&mut game, place(pos![4, 1])).expect("the pawn is chosen");
        assert_eq!(game.current_player(), "WHITE", "nothing is played yet");
        choose(&mut game, place(pos![4, 3])).expect("the pawn goes");

        assert_eq!(code_at(&game, pos![4, 3]).as_deref(), Some("PAWN"));
        assert_eq!(code_at(&game, pos![4, 1]), None);
        assert_eq!(game.current_player(), "BLACK");
        assert_eq!(game.state.phase, GamePhase::Choosing { chosen: Vec::new() });
    }

    #[test]
    fn the_choice_lists_movable_pieces_then_destinations() {
        let mut game = chess();

        let start = game.choice();
        assert!(start.chosen.is_empty());
        assert_eq!(start.start, None);
        let mut pieces: Vec<Position> = start.places.iter().map(|p| p.name).collect();
        pieces.sort();
        let mut movable: Vec<Position> = (0..8).map(|x| pos![x, 1]).collect();
        movable.extend([pos![1, 0], pos![6, 0]]);
        movable.sort();
        assert_eq!(pieces, movable, "every pawn and both knights");
        assert!(start.places.iter().all(|p| p.r#for == ChoiceKind::Piece));
        assert!(start.codes.is_empty() && start.hand.is_empty() && start.actions.is_empty());

        choose(&mut game, place(pos![4, 1])).expect("the pawn is chosen");
        let picked = game.choice();
        assert_eq!(picked.chosen, vec![place(pos![4, 1])]);
        assert_eq!(picked.start, Some(pos![4, 1]));
        assert_eq!(
            picked.places,
            vec![
                PlaceOption {
                    name: pos![4, 2],
                    r#for: ChoiceKind::Move
                },
                PlaceOption {
                    name: pos![4, 3],
                    r#for: ChoiceKind::Move
                },
            ]
        );
        let written = serde_json::to_value(&picked.places[0]).expect("written");
        assert_eq!(written, serde_json::json!({ "name": "4,2", "for": "MOVE" }));
    }

    #[test]
    fn a_promotion_shows_the_moved_pawn_and_cannot_be_cancelled() {
        let mut game = about_to_promote();

        choose(&mut game, place(pos![0, 6])).expect("the pawn is chosen");
        assert_eq!(
            game.shown_pieces(),
            game.state.pieces,
            "nothing is shown moved before the place is chosen"
        );
        choose(&mut game, place(pos![0, 7])).expect("the pawn reaches the last rank");

        let choice = game.choice();
        assert_eq!(choice.codes, vec!["QUEEN", "ROOK", "BISHOP", "KNIGHT"]);
        assert!(choice.places.is_empty());
        assert_eq!(game.current_player(), "WHITE", "the promotion is pending");
        let shown = game.shown_pieces();
        assert_eq!(
            shown.get(&pos![0, 7]).map(|p| p.code.as_str()),
            Some("PAWN")
        );
        assert!(!shown.contains_key(&pos![0, 6]), "the pawn has left");
        assert_eq!(
            code_at(&game, pos![0, 6]).as_deref(),
            Some("PAWN"),
            "not played"
        );

        let before = state_json(&game);
        assert!(matches!(cancel(&mut game), Err(GameError::CannotCancel)));
        assert_eq!(state_json(&game), before);

        choose(&mut game, code("ROOK")).expect("the promotion is answered");
        assert_eq!(code_at(&game, pos![0, 7]).as_deref(), Some("ROOK"));
        assert_eq!(game.current_player(), "BLACK");
        assert_eq!(game.shown_pieces(), game.state.pieces);
    }

    #[test]
    fn cancel_before_any_effect_clears_the_answers() {
        let mut game = chess();
        choose(&mut game, place(pos![4, 1])).expect("the pawn is chosen");

        cancel(&mut game).expect("nothing has been shown yet");

        assert!(chosen(&game).is_empty());
        assert_eq!(game.choice().start, None);
        choose(&mut game, place(pos![6, 0])).expect("another piece can be chosen");
        choose(&mut game, place(pos![5, 2])).expect("and played");
        assert_eq!(code_at(&game, pos![5, 2]).as_deref(), Some("KNIGHT"));
        assert_eq!(code_at(&game, pos![4, 1]).as_deref(), Some("PAWN"));
    }

    #[test]
    fn an_action_name_narrows_and_a_pass_plays_at_once() {
        let mut game = load("./specs/go.json");

        let start = game.choice();
        assert_eq!(
            start.actions,
            vec![
                ActionOption {
                    name: "STONE".to_string(),
                    plays: false
                },
                ActionOption {
                    name: "PASS".to_string(),
                    plays: true
                },
            ]
        );
        assert_eq!(start.places.len(), 81);
        assert!(start.places.iter().all(|p| p.r#for == ChoiceKind::Place));

        choose(&mut game, action("STONE")).expect("the stone is chosen");
        let narrowed = game.choice();
        assert_eq!(narrowed.chosen, vec![action("STONE")]);
        assert_eq!(narrowed.places.len(), 81);
        assert!(narrowed.actions.is_empty(), "only one action is left");
        assert!(matches!(
            choose(&mut game, action("PASS")),
            Err(GameError::NotAnOption)
        ));

        cancel(&mut game).expect("nothing has been shown yet");
        choose(&mut game, action("PASS")).expect("the pass is played");
        assert_eq!(game.current_player(), "WHITE");
        assert!(game.history().last().is_some_and(|record| record.passed));

        choose(&mut game, place(pos![4, 4])).expect("one way: the stone is played at once");
        assert_eq!(code_at(&game, pos![4, 4]).as_deref(), Some("STONE"));
        assert_eq!(game.current_player(), "BLACK");
    }

    #[test]
    fn a_shorter_complete_way_is_offered_beside_longer_ones() {
        let mut game = with_pieces(load(WAYS), &[(pos![3, 3], "PAWN", "WHITE")]);

        choose(&mut game, place(pos![3, 3])).expect("the pawn is chosen");

        let choice = game.choice();
        assert_eq!(
            choice.places,
            vec![PlaceOption {
                name: pos![3, 4],
                r#for: ChoiceKind::Move
            }]
        );
        assert_eq!(
            choice.actions,
            vec![
                ActionOption {
                    name: "MOVE".to_string(),
                    plays: false
                },
                ActionOption {
                    name: "PROMOTE".to_string(),
                    plays: true
                },
            ]
        );
        assert_eq!(game.current_player(), "WHITE", "nothing is played yet");

        let mut moved = game.clone();
        choose(&mut moved, place(pos![3, 4])).expect("the move is played");
        assert_eq!(code_at(&moved, pos![3, 4]).as_deref(), Some("PAWN"));

        choose(&mut game, action("PROMOTE")).expect("the shorter way is played");
        assert_eq!(code_at(&game, pos![3, 3]).as_deref(), Some("GOLD"));
        assert_eq!(game.current_player(), "BLACK");
    }

    #[test]
    fn the_jump_lock_starts_the_next_turn_with_the_piece_chosen_and_refuses_cancel() {
        let mut game = double_jump();

        choose(&mut game, place(pos![3, 1])).expect("the man is chosen");
        choose(&mut game, place(pos![5, 3])).expect("the first jump is played");

        assert_eq!(game.current_player(), "RED", "the same player goes on");
        assert_eq!(game.state.continuing, Some(pos![5, 3]));
        let choice = game.choice();
        assert_eq!(choice.chosen, vec![place(pos![5, 3])]);
        assert_eq!(choice.start, Some(pos![5, 3]));
        assert_eq!(
            choice.places,
            vec![PlaceOption {
                name: pos![3, 5],
                r#for: ChoiceKind::Move
            }]
        );

        let before = state_json(&game);
        assert!(matches!(cancel(&mut game), Err(GameError::CannotCancel)));
        assert!(matches!(
            choose(&mut game, place(pos![0, 1])),
            Err(GameError::NotAnOption)
        ));
        assert_eq!(state_json(&game), before);

        choose(&mut game, place(pos![3, 5])).expect("the second jump is played");
        assert_eq!(game.current_player(), "BLACK");
        assert_eq!(game.state.continuing, None);
        assert!(chosen(&game).is_empty());
        assert_eq!(code_at(&game, pos![4, 4]), None);
    }

    #[test]
    fn end_turn_stops_the_jump_continuation() {
        let spec = || parse_game_spec_str(JUMPS).expect("the spec is read");

        // Straight ahead, a second jump follows.
        let mut ahead = with_pieces(
            Game::from_spec(spec()),
            &[
                (pos![0, 0], "MAN", "WHITE"),
                (pos![0, 1], "MAN", "BLACK"),
                (pos![0, 3], "MAN", "BLACK"),
                (pos![7, 7], "MAN", "BLACK"),
            ],
        );
        choose(&mut ahead, place(pos![0, 0])).expect("the man is chosen");
        choose(&mut ahead, place(pos![0, 2])).expect("the jump is played");
        assert_eq!(ahead.state.continuing, Some(pos![0, 2]));
        assert_eq!(ahead.current_player(), "WHITE");

        // To the right, a jump ends the turn although a second one is there.
        let mut right = with_pieces(
            Game::from_spec(spec()),
            &[
                (pos![0, 0], "MAN", "WHITE"),
                (pos![1, 0], "MAN", "BLACK"),
                (pos![3, 0], "MAN", "BLACK"),
                (pos![7, 7], "MAN", "BLACK"),
            ],
        );
        choose(&mut right, place(pos![0, 0])).expect("the man is chosen");
        choose(&mut right, place(pos![2, 0])).expect("the jump is played");
        assert_eq!(right.state.continuing, None);
        assert_eq!(right.current_player(), "BLACK");
        assert!(chosen(&right).is_empty());
    }

    #[test]
    fn answers_that_are_not_options_change_nothing() {
        let mut game = chess();
        choose(&mut game, place(pos![4, 1])).expect("the pawn is chosen");
        let before = state_json(&game);
        let choice = game.choice();

        for answer in [
            code("QUEEN"),
            place(Position::from("99,99")),
            place(Position::from("nowhere")),
            Answer::Hand("PAWN".to_string()),
            action("FLY"),
        ] {
            let result = choose(&mut game, answer.clone());
            assert!(
                matches!(result, Err(GameError::NotAnOption)),
                "{answer:?}: {result:?}"
            );
            assert_eq!(state_json(&game), before, "{answer:?} changed the state");
            assert_eq!(game.choice(), choice, "{answer:?} changed the choice");
        }
    }

    #[test]
    fn a_restored_game_keeps_its_answers_and_its_jump_lock() {
        let mut picked = chess();
        choose(&mut picked, place(pos![4, 1])).expect("the pawn is chosen");
        let mut again = chess();
        again
            .restore_state(&state_json(&picked))
            .expect("the state is read");
        assert_eq!(again.choice(), picked.choice());
        choose(&mut again, place(pos![4, 3])).expect("the restored game plays on");
        assert_eq!(code_at(&again, pos![4, 3]).as_deref(), Some("PAWN"));

        let mut jumping = double_jump();
        choose(&mut jumping, place(pos![3, 1])).expect("the man is chosen");
        choose(&mut jumping, place(pos![5, 3])).expect("the first jump is played");
        let mut resumed = load("./specs/checkers.json");
        resumed
            .restore_state(&state_json(&jumping))
            .expect("the state is read");
        assert_eq!(resumed.state.continuing, Some(pos![5, 3]));
        assert_eq!(resumed.choice(), jumping.choice());
        assert!(matches!(cancel(&mut resumed), Err(GameError::CannotCancel)));
        choose(&mut resumed, place(pos![3, 5])).expect("the jump goes on");
        assert_eq!(resumed.current_player(), "BLACK");
    }

    #[test]
    fn the_history_records_the_action_its_answers_and_captures() {
        let mut game = chess();
        choose(&mut game, place(pos![4, 1])).expect("chosen");
        choose(&mut game, place(pos![4, 3])).expect("played");

        let record = game.history().last().expect("a record").clone();
        assert_eq!(record.player, "WHITE");
        assert_eq!(record.action, "MOVE");
        assert_eq!(record.chosen, vec![place(pos![4, 1]), place(pos![4, 3])]);
        assert_eq!(record.piece_code, "PAWN");
        assert_eq!((record.from, record.to), (pos![4, 1], pos![4, 3]));
        assert!(!record.captures && !record.passed);

        // A promotion keeps its code among the answers.
        let mut promoting = about_to_promote();
        choose(&mut promoting, place(pos![0, 6])).expect("chosen");
        choose(&mut promoting, place(pos![0, 7])).expect("moved");
        choose(&mut promoting, code("QUEEN")).expect("promoted");
        let record = promoting.history().last().expect("a record").clone();
        assert_eq!(
            record.chosen,
            vec![place(pos![0, 6]), place(pos![0, 7]), code("QUEEN")]
        );
        assert_eq!(record.piece_code, "PAWN");

        // A capture by an action that places a stone and removes the piece beside it.
        let mut striking = with_pieces(load(WAYS), &[(pos![4, 2], "ROOK", "BLACK")]);
        choose(&mut striking, action("STRIKE")).expect("the action is chosen");
        choose(&mut striking, place(pos![3, 2])).expect("the stone is placed");
        let record = striking.history().last().expect("a record").clone();
        assert_eq!(record.action, "STRIKE");
        assert_eq!(record.chosen, vec![place(pos![3, 2])]);
        assert_eq!(record.piece_code, "STONE");
        assert_eq!((record.from, record.to), (Position::default(), pos![3, 2]));
        assert!(record.captures);
        assert_eq!(code_at(&striking, pos![4, 2]), None);
    }

    #[test]
    fn the_fifty_move_rule_resets_on_a_capture_way() {
        // Kings, a WHITE rook and a BLACK knight it can take, after 99 quiet half-moves.
        let quiet = || {
            let mut game = with_pieces(
                chess(),
                &[
                    (pos![4, 0], "KING", "WHITE"),
                    (pos![7, 7], "KING", "BLACK"),
                    (pos![0, 0], "ROOK", "WHITE"),
                    (pos![0, 5], "KNIGHT", "BLACK"),
                ],
            );
            for player in ["WHITE", "BLACK"] {
                game.state
                    .counters
                    .entry(player.to_string())
                    .or_default()
                    .insert("QUIET".to_string(), 99);
            }
            game
        };

        let mut taking = quiet();
        choose(&mut taking, place(pos![0, 0])).expect("chosen");
        choose(&mut taking, place(pos![0, 5])).expect("the knight is taken");
        assert!(taking.history().last().is_some_and(|r| r.captures));
        assert!(
            matches!(taking.state.phase, GamePhase::Choosing { .. }),
            "a capture resets the count: {:?}",
            taking.state.phase
        );

        let mut waiting = quiet();
        choose(&mut waiting, place(pos![0, 0])).expect("chosen");
        choose(&mut waiting, place(pos![0, 1])).expect("a quiet move");
        assert_eq!(waiting.state.phase, GamePhase::GameOver { winner: None });
    }

    #[test]
    fn two_passes_in_a_row_are_counted() {
        let mut game = load("./specs/go.json");
        choose(&mut game, action("PASS")).expect("BLACK passes");
        assert_eq!(game.consecutive_passes(), 1);
        choose(&mut game, place(pos![4, 4])).expect("WHITE places a stone");
        assert_eq!(game.consecutive_passes(), 0, "a stone breaks the run");

        choose(&mut game, action("PASS")).expect("BLACK passes");
        choose(&mut game, action("PASS")).expect("WHITE passes");
        assert_eq!(game.consecutive_passes(), 2);
        let last_two: Vec<bool> = game
            .history()
            .iter()
            .rev()
            .take(2)
            .map(|r| r.passed)
            .collect();
        assert_eq!(last_two, vec![true, true]);
        assert!(matches!(game.state.phase, GamePhase::GameOver { .. }));
    }
}
