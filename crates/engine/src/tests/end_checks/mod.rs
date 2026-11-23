//! End checks and the score: what ends a game, who wins, and what a player who has no way
//! to play meets; with their refusals when the spec is read.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Game, GamePhase, Piece};
    use crate::shared::Position;
    use crate::specs::{parse_game_spec_str, GameSpecError};
    use crate::tests::support::{go, pass, pass_action, pick};

    fn at(place: &str) -> Value {
        json!({ "place": place })
    }

    fn player(name: &str, direction: Value) -> Value {
        json!({ "name": name, "direction": direction, "starting_positions": [] })
    }

    fn move_action() -> Value {
        json!({ "name": "MOVE", "do": [
            { "choose": "PIECE", "where": { "owner": "ALLY" } },
            { "choose": "MOVE" }
        ] })
    }

    /// A 4×4 game of two players. NUDGE steps one square along the file either way onto a
    /// square no ally holds; PAWN and KING do not move.
    fn spec(actions: Value, end_checks: Value) -> Value {
        let nudge = |id: u8, step: [i8; 2]| {
            json!({ "id": id, "step": step, "do": [
                { "test": "NOT", "of": { "test": "COUNT",
                    "in": { "region": "AT", "at": at("TARGET") },
                    "where": { "owner": "ALLY" } } },
                { "effect": "MOVE", "from": at("START"), "to": at("TARGET") }
            ] })
        };
        json!({
            "name": "END_CHECKS",
            "board": { "dimensions": [4, 4] },
            "players": [
                player("WHITE", json!([[1, 0], [0, 1]])),
                player("BLACK", json!([[-1, 0], [0, -1]]))
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "pieces": [
                { "code": "NUDGE", "name": "nudge", "moves": [nudge(0, [0, 1]), nudge(1, [0, -1])] },
                { "code": "PAWN", "name": "pawn", "moves": [] },
                { "code": "KING", "name": "king", "moves": [] }
            ],
            "actions": actions,
            "end_checks": end_checks
        })
    }

    fn game_of(spec: &Value, pieces: &[(Position, &str, &str)]) -> Game {
        let spec = parse_game_spec_str(&spec.to_string())
            .unwrap_or_else(|error| panic!("the spec is read: {error}"));
        let mut game = Game::from_spec(spec);
        game.state.pieces.clear();
        for (place, code, owner) in pieces {
            game.state.pieces.insert(
                *place,
                Piece::new((*code).to_string(), (*owner).to_string()),
            );
        }
        game.work_out_ways();
        game
    }

    fn winner(game: &Game) -> Option<Option<String>> {
        match &game.state.phase {
            GamePhase::GameOver { winner } => Some(winner.clone()),
            GamePhase::Choosing { .. } => None,
        }
    }

    fn white_moves(game: &mut Game) {
        pick(game, pos![0, 0]).unwrap();
        go(game, pos![0, 1]).unwrap();
    }

    const WHITE: &str = "WHITE";
    const BLACK: &str = "BLACK";

    fn count(owner: &str, codes: Value, min: Option<u16>, max: Option<u16>) -> Value {
        let mut test = json!({ "test": "COUNT", "in": { "region": "BOARD" },
            "where": { "owner": owner, "codes": codes } });
        if let Some(min) = min {
            test["min"] = json!(min);
        } else if max.is_some() {
            // A COUNT's min is 1 when not written.
            test["min"] = json!(0);
        }
        if let Some(max) = max {
            test["max"] = json!(max);
        }
        test
    }

    #[test]
    fn win_ends_the_game_for_the_mover() {
        let checks = json!([{ "when": count("ALLY", json!(["NUDGE"]), Some(2), None),
            "then": { "outcome": "WIN" } }]);
        let mut game = game_of(
            &spec(json!([move_action()]), checks),
            &[
                (pos![0, 0], "NUDGE", WHITE),
                (pos![2, 0], "NUDGE", WHITE),
                (pos![3, 3], "NUDGE", BLACK),
            ],
        );
        white_moves(&mut game);
        assert_eq!(winner(&game), Some(Some(WHITE.to_string())));
    }

    #[test]
    fn a_check_that_does_not_hold_leaves_the_game_going() {
        let checks = json!([{ "when": count("ALLY", json!(["NUDGE"]), Some(3), None),
            "then": { "outcome": "WIN" } }]);
        let mut game = game_of(
            &spec(json!([move_action()]), checks),
            &[(pos![0, 0], "NUDGE", WHITE), (pos![3, 3], "NUDGE", BLACK)],
        );
        white_moves(&mut game);
        assert_eq!(winner(&game), None);
        assert_eq!(game.current_player(), BLACK);
    }

    #[test]
    fn draw_ends_the_game_with_no_winner() {
        let checks = json!([{ "when": count("ENEMY", json!([]), None, Some(0)),
            "then": { "outcome": "DRAW" } }]);
        let mut game = game_of(
            &spec(json!([move_action()]), checks),
            &[(pos![0, 0], "NUDGE", WHITE)],
        );
        white_moves(&mut game);
        assert_eq!(winner(&game), Some(None));
    }

    #[test]
    fn the_first_check_that_holds_decides_and_later_checks_do_not_run() {
        let holds = count("ALLY", json!(["NUDGE"]), Some(1), None);
        let checks = json!([
            { "when": holds.clone(), "then": { "outcome": "WIN" } },
            { "when": holds, "then": { "outcome": "DRAW" } }
        ]);
        let mut game = game_of(
            &spec(json!([move_action()]), checks),
            &[(pos![0, 0], "NUDGE", WHITE), (pos![3, 3], "NUDGE", BLACK)],
        );
        white_moves(&mut game);
        assert_eq!(winner(&game), Some(Some(WHITE.to_string())));
    }

    #[test]
    fn as_next_reads_ally_as_the_player_about_to_play() {
        let checks = json!([{ "as": "NEXT", "when": count("ALLY", json!(["KING"]), Some(1), None),
            "then": { "outcome": "WIN" } }]);
        let mut game = game_of(
            &spec(json!([move_action()]), checks),
            &[
                (pos![0, 0], "NUDGE", WHITE),
                (pos![3, 3], "KING", BLACK),
                (pos![3, 0], "NUDGE", BLACK),
            ],
        );
        white_moves(&mut game);
        assert_eq!(
            winner(&game),
            Some(Some(BLACK.to_string())),
            "BLACK is next"
        );
    }

    #[test]
    fn lose_removes_the_next_player_and_ends_a_two_player_game() {
        let checks = json!([{ "as": "NEXT", "when": count("ALLY", json!([]), None, Some(0)),
            "then": { "outcome": "LOSE" } }]);
        let mut game = game_of(
            &spec(json!([move_action()]), checks),
            &[(pos![0, 0], "NUDGE", WHITE)],
        );
        white_moves(&mut game);
        assert_eq!(game.state.turn_order, vec![WHITE.to_string()]);
        assert_eq!(winner(&game), Some(Some(WHITE.to_string())));
    }

    fn four_players(checks: Value) -> Value {
        let mut value = spec(json!([move_action()]), checks);
        value["players"] = json!([
            player("A", json!([[1, 0], [0, 1]])),
            player("B", json!([[-1, 0], [0, -1]])),
            player("C", json!([[1, 0], [0, 1]])),
            player("D", json!([[-1, 0], [0, -1]]))
        ]);
        value["turns"] = json!({ "order": ["A", "B", "C", "D"] });
        value
    }

    #[test]
    fn lose_in_four_players_keeps_the_rest_playing_with_a_valid_cursor() {
        let checks = json!([{ "as": "NEXT", "when": count("ALLY", json!([]), None, Some(0)),
            "then": { "outcome": "LOSE" } }]);
        let mut game = game_of(
            &four_players(checks),
            &[
                (pos![0, 0], "NUDGE", "A"),
                (pos![3, 3], "NUDGE", "B"),
                (pos![0, 3], "NUDGE", "D"),
            ],
        );
        white_moves(&mut game);
        pick(&mut game, pos![3, 3]).unwrap();
        go(&mut game, pos![3, 2]).unwrap();
        // C has no piece: it is out, and D, who comes after, is to play.
        assert_eq!(game.state.turn_order, ["A", "B", "D"]);
        assert_eq!(game.current_player(), "D");
        assert_eq!(winner(&game), None);
        assert!(game.any_legal_moves());
    }

    #[test]
    fn losing_as_the_mover_keeps_the_player_to_move_to_move() {
        let checks = json!([{ "when": count("ALLY", json!(["PAWN"]), None, Some(0)),
            "then": { "outcome": "LOSE" } }]);
        let mut game = game_of(
            &four_players(checks),
            &[
                (pos![0, 0], "NUDGE", "A"),
                (pos![0, 3], "PAWN", "A"),
                (pos![3, 3], "NUDGE", "B"),
                (pos![0, 2], "NUDGE", "C"),
                (pos![2, 3], "NUDGE", "D"),
            ],
        );
        white_moves(&mut game); // A keeps its pawn.
        assert_eq!(game.state.turn_order.len(), 4);
        pick(&mut game, pos![3, 3]).unwrap();
        go(&mut game, pos![3, 2]).unwrap(); // B has no pawn: out.
        assert_eq!(game.state.turn_order, ["A", "C", "D"]);
        assert_eq!(game.current_player(), "C", "C was next, and still is");
    }

    #[test]
    fn stuck_as_next_ends_the_game_only_when_the_player_to_move_has_no_way() {
        let checks = json!([{ "as": "NEXT", "when": { "test": "STUCK" },
            "then": { "outcome": "DRAW" } }]);
        let mut game = game_of(
            &spec(json!([move_action()]), checks.clone()),
            &[(pos![0, 0], "NUDGE", WHITE), (pos![3, 3], "NUDGE", BLACK)],
        );
        white_moves(&mut game);
        assert_eq!(winner(&game), None, "BLACK has a way");
        assert!(game.any_legal_moves());

        let mut game = game_of(
            &spec(json!([move_action()]), checks),
            &[(pos![0, 0], "NUDGE", WHITE), (pos![3, 3], "PAWN", BLACK)],
        );
        white_moves(&mut game);
        assert_eq!(winner(&game), Some(None), "BLACK has no way");
    }

    #[test]
    fn with_no_way_and_no_check_that_holds_the_game_is_a_draw() {
        let mut game = game_of(
            &spec(json!([move_action()]), json!([])),
            &[(pos![0, 0], "NUDGE", WHITE), (pos![3, 3], "PAWN", BLACK)],
        );
        white_moves(&mut game);
        assert_eq!(winner(&game), Some(None));
    }

    fn passing(score: Value) -> Value {
        let mut value = spec(
            json!([pass_action()]),
            json!([{ "when": { "test": "PASSES", "in_a_row": 2 },
                "then": { "outcome": "HIGHEST" } }]),
        );
        value["score"] = score;
        value
    }

    fn two_passes(score: Value) -> Game {
        let mut game = game_of(
            &passing(score),
            &[(pos![0, 0], "NUDGE", WHITE), (pos![3, 3], "NUDGE", BLACK)],
        );
        pass(&mut game).unwrap();
        assert_eq!(winner(&game), None, "one pass does not end the game");
        pass(&mut game).unwrap();
        game
    }

    #[test]
    fn highest_gives_the_game_to_the_best_score_and_a_tie_is_a_draw() {
        let pieces = json!({ "term": "PIECES", "where": { "codes": ["NUDGE"] } });
        let tied = two_passes(json!([pieces.clone()]));
        assert_eq!(winner(&tied), Some(None));
        let komi = json!({ "term": "POINTS", "value": 0.5, "to": ["BLACK"] });
        let komi_wins = two_passes(json!([pieces, komi]));
        assert_eq!(winner(&komi_wins), Some(Some(BLACK.to_string())));
    }

    #[test]
    fn the_score_is_shown_live_and_a_game_without_one_has_none() {
        let pieces = json!({ "term": "PIECES" });
        let counter = json!({ "term": "COUNTER", "name": "N" });
        let mut value = passing(json!([pieces, counter]));
        value["counters"] = json!(["N"]);
        let mut game = game_of(
            &value,
            &[
                (pos![0, 0], "NUDGE", WHITE),
                (pos![1, 1], "PAWN", WHITE),
                (pos![3, 3], "NUDGE", BLACK),
            ],
        );
        game.state
            .counters
            .insert(BLACK.into(), [("N".into(), 4)].into());
        let scores = game.scores().expect("the game has a score");
        assert_eq!((scores[WHITE], scores[BLACK]), (2.0, 5.0));
        let plain = game_of(&spec(json!([pass_action()]), json!([])), &[]);
        assert!(plain.scores().is_none());
    }

    #[test]
    fn territory_counts_the_empty_regions_one_player_alone_touches() {
        let mut value = spec(json!([pass_action()]), json!([]));
        value["score"] = json!([{ "term": "TERRITORY", "where": { "codes": ["PAWN"] } }]);
        // A wall of WHITE pawns on file 1 cuts off file 0 (4 empty places, WHITE's alone); a
        // BLACK pawn on 3,3 touches the rest, which also touches WHITE's wall: shared, nobody's.
        let game = game_of(
            &value,
            &[
                (pos![1, 0], "PAWN", WHITE),
                (pos![1, 1], "PAWN", WHITE),
                (pos![1, 2], "PAWN", WHITE),
                (pos![1, 3], "PAWN", WHITE),
                (pos![3, 3], "PAWN", BLACK),
            ],
        );
        let scores = game.scores().expect("the game has a score");
        assert_eq!((scores[WHITE], scores[BLACK]), (4.0, 0.0));
    }

    #[test]
    fn repeats_in_an_end_check_counts_the_position_just_reached() {
        // The second check reads `moved`, so the has-moved flag is part of the record: the
        // pieces' first steps make the positions after plies 2, 6 and 10 the same (both pieces
        // moved, WHITE to play), and the third one ends the game.
        let checks = json!([
            { "when": { "test": "REPEATS", "times": 3 }, "then": { "outcome": "DRAW" } },
            { "when": { "test": "COUNT", "in": { "region": "BOARD" },
                "where": { "moved": true, "codes": ["KING"] }, "min": 5 },
              "then": { "outcome": "DRAW" } }
        ]);
        let mut game = game_of(
            &spec(json!([move_action()]), checks),
            &[(pos![0, 0], "NUDGE", WHITE), (pos![3, 3], "NUDGE", BLACK)],
        );
        let cycle = [
            (pos![0, 0], pos![0, 1]),
            (pos![3, 3], pos![3, 2]),
            (pos![0, 1], pos![0, 0]),
            (pos![3, 2], pos![3, 3]),
        ];
        for ply in 0..10 {
            let (from, to) = &cycle[ply % 4];
            assert_eq!(winner(&game), None, "before ply {}", ply + 1);
            pick(&mut game, *from).unwrap();
            go(&mut game, *to).unwrap();
        }
        assert_eq!(winner(&game), Some(None), "the third time, at ply 10");
    }

    fn refused(value: Value) -> String {
        match parse_game_spec_str(&value.to_string()) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("the spec was not refused"),
        }
    }

    #[test]
    fn an_unknown_outcome_or_viewpoint_is_refused() {
        let check = |as_: Option<&str>, outcome: &str| {
            let mut check = json!({ "when": count("ALLY", json!([]), None, None),
                "then": { "outcome": outcome } });
            if let Some(as_) = as_ {
                check["as"] = json!(as_);
            }
            spec(json!([pass_action()]), json!([check]))
        };
        assert!(refused(check(None, "CONCEDE")).contains("CONCEDE"));
        assert!(refused(check(Some("PREVIOUS"), "WIN")).contains("PREVIOUS"));
        let mut extra = check(None, "WIN");
        extra["end_checks"][0]["then"]["player"] = json!("WHITE");
        assert!(refused(extra).contains("player"));
    }

    #[test]
    fn stuck_is_refused_as_mover_and_outside_an_end_check() {
        let as_mover = spec(
            json!([pass_action()]),
            json!([{ "when": { "test": "STUCK" }, "then": { "outcome": "DRAW" } }]),
        );
        let message = refused(as_mover);
        assert!(
            message.contains("STUCK") && message.contains("NEXT"),
            "{message}"
        );
        let in_a_step = spec(
            json!([{ "name": "A", "do": [{ "test": "STUCK" }] }]),
            json!([]),
        );
        assert!(refused(in_a_step).contains("STUCK"));
    }

    #[test]
    fn an_end_check_names_no_place_and_no_removed() {
        let place = json!({ "test": "COUNT", "in": { "region": "AT", "at": at("TARGET") } });
        let removed = json!({ "test": "REMOVED", "where": { "owner": "ENEMY" } });
        for test in [place, removed] {
            let value = spec(
                json!([pass_action()]),
                json!([{ "when": test, "then": { "outcome": "DRAW" } }]),
            );
            let message = refused(value);
            assert!(message.contains("end_checks"), "{message}");
        }
    }

    #[test]
    fn highest_without_a_score_and_bad_score_terms_are_refused() {
        let value = spec(
            json!([pass_action()]),
            json!([{ "when": { "test": "PASSES", "in_a_row": 1 },
                "then": { "outcome": "HIGHEST" } }]),
        );
        assert!(refused(value).contains("score"));
        let mut owned = passing(json!([{ "term": "PIECES", "where": { "owner": "ALLY" } }]));
        owned["score"][0]["where"]["owner"] = json!("ALLY");
        assert!(refused(owned).contains("owner"));
        let stranger = passing(json!([{ "term": "POINTS", "value": 1.0, "to": ["GREEN"] }]));
        assert!(refused(stranger).contains("GREEN"));
        let undeclared = passing(json!([{ "term": "COUNTER", "name": "N" }]));
        assert!(refused(undeclared).contains("`N`"));
        let _ = GameSpecError::EmptyTurnOrder;
    }
}
