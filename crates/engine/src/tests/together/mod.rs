//! Answers out of turn, all at the same time: the `ASK` effect with `TOGETHER`, sealed or open.
//! How a whole game plays with it is proved by rock-paper-scissors, in the session crate's tests
//! and the game server's; this is what the effect does and what the spec may and may not say.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Answer, Game, GamePhase};
    use crate::specs::{parse_game_spec_str, GameSpec, GameSpecError};

    fn read(spec: &Value) -> Result<GameSpec, GameSpecError> {
        parse_game_spec_str(&spec.to_string())
    }

    fn refusal(spec: &Value) -> String {
        read(spec).expect_err("the spec is refused").to_string()
    }

    fn accepted(spec: &Value) {
        if let Err(error) = read(spec) {
            panic!("the spec is refused: {error}");
        }
    }

    /// Three players, `A`, `B` and `C`, on a board of four squares. `CALL` asks as `ask` says; in
    /// `REPLY` a seat chooses a code and a square and puts that piece there, only on an empty one.
    fn spec_asking(ask: Value) -> Value {
        json!({
            "format_version": 1,
            "name": "TOGETHER",
            "board": { "dimensions": [2, 2] },
            "players": [
                { "name": "A", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "B", "direction": [[-1, 0], [0, -1]], "starting_positions": [] },
                { "name": "C", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["A", "B", "C"],
                       "stages": [{ "name": "REPLY" }, { "name": "AFTER" }] },
            "pieces": [{ "code": "P", "name": "p", "moves": [] },
                       { "code": "Q", "name": "q", "moves": [] }],
            "counters": ["SEEN"],
            "actions": [
                { "name": "CALL", "do": [ask] },
                { "name": "PUT", "stage": "REPLY", "do": [
                    { "choose": "CODE", "as": "WHAT", "options": ["P", "Q"] },
                    { "choose": "PLACE", "in": { "region": "BOARD" } },
                    { "test": "NOT", "of": { "test": "COUNT",
                        "in": { "region": "AT", "at": { "place": "TARGET" } } } },
                    { "effect": "PLACE", "at": { "place": "TARGET" }, "code": { "chosen": "WHAT" } }
                ] },
                { "name": "FOLLOW", "stage": "AFTER", "do": [] }
            ]
        })
    }

    fn ask(players: Value, sealed: Option<bool>, then: bool) -> Value {
        let mut ask = json!({ "effect": "ASK", "players": players, "stage": "REPLY",
                              "mode": "TOGETHER" });
        if let Some(sealed) = sealed {
            ask["sealed"] = json!(sealed);
        }
        if then {
            ask["then"] = json!({ "stage": "AFTER" });
        }
        ask
    }

    fn build(spec: &Value) -> Game {
        Game::from_spec(read(spec).expect("the spec reads"))
    }

    fn act(game: &mut Game, name: &str) {
        game.choose(Answer::Action(name.to_string()))
            .unwrap_or_else(|error| panic!("{name} is refused: {error:?}"));
    }

    /// `seat` answers: the code, then the square.
    fn put(game: &mut Game, seat: &str, code: &str, at: &str) {
        game.choose_as(seat, Answer::Code(code.to_string()))
            .unwrap_or_else(|error| panic!("{seat} cannot choose {code}: {error:?}"));
        game.choose_as(seat, Answer::Place(at.into()))
            .unwrap_or_else(|error| panic!("{seat} cannot choose {at}: {error:?}"));
    }

    fn on_board(game: &Game) -> Vec<String> {
        let mut pieces: Vec<String> = game
            .state
            .pieces
            .iter()
            .map(|(at, piece)| format!("{at}={}{}", piece.player, piece.code))
            .collect();
        pieces.sort();
        pieces
    }

    fn log(game: &Game) -> Vec<String> {
        game.history()
            .iter()
            .map(|record| format!("{} {}", record.player, record.action))
            .collect()
    }

    fn places_for(game: &Game, seat: &str) -> Vec<String> {
        game.focused(seat)
            .expect("the seat owes an answer")
            .choice()
            .places
            .iter()
            .map(|place| place.name.as_str().to_string())
            .collect()
    }

    #[test]
    fn every_asked_seat_owes_an_answer_at_once_and_nobody_is_to_move() {
        let mut game = build(&spec_asking(ask(json!("OTHERS"), None, false)));
        act(&mut game, "CALL");
        assert_eq!(game.waiting(), ["B", "C"]);
        assert!(game.owes_answer("B") && game.owes_answer("C") && !game.owes_answer("A"));
        // Both may choose now, whichever of them does it first.
        assert!(game.focused("C").is_some());
        assert!(game.focused("A").is_none(), "the asker owes nothing");
        assert!(game.choose(Answer::Action("CALL".into())).is_err());
        assert!(!game.any_legal_moves());
    }

    #[test]
    fn a_sealed_answer_is_kept_and_nothing_is_played_until_every_seat_has_answered() {
        let mut game = build(&spec_asking(ask(json!("OTHERS"), None, false)));
        act(&mut game, "CALL");
        put(&mut game, "B", "P", "0,0");
        assert_eq!(game.waiting(), ["C"], "B has answered");
        assert!(on_board(&game).is_empty(), "nothing is played yet");
        assert_eq!(log(&game), ["A CALL"], "nor recorded");
        // C chooses on the position both were asked on: B's square is still open to it.
        game.choose_as("C", Answer::Code("Q".into())).unwrap();
        assert!(places_for(&game, "C").contains(&"0,0".to_string()));
        game.cancel_as("C").unwrap();
        assert!(
            game.choose_as("B", Answer::Code("P".into())).is_err(),
            "B has answered"
        );
        put(&mut game, "C", "Q", "1,1");
        assert_eq!(on_board(&game), ["0,0=BP", "1,1=CQ"]);
        assert_eq!(log(&game), ["A CALL", "B PUT", "C PUT"].map(String::from));
        assert!(!game.together_pending());
        assert_eq!(
            game.current_player(),
            "B",
            "the turn is handed on after the asker"
        );
    }

    #[test]
    fn the_seats_may_answer_in_either_order_and_play_in_the_order_they_were_asked() {
        let mut first = build(&spec_asking(ask(json!("OTHERS"), None, false)));
        act(&mut first, "CALL");
        put(&mut first, "B", "P", "0,0");
        put(&mut first, "C", "Q", "1,1");
        let mut second = build(&spec_asking(ask(json!("OTHERS"), None, false)));
        act(&mut second, "CALL");
        put(&mut second, "C", "Q", "1,1");
        put(&mut second, "B", "P", "0,0");
        assert_eq!(on_board(&first), on_board(&second));
        assert_eq!(
            log(&first),
            log(&second),
            "B's answer comes first either way"
        );
    }

    #[test]
    fn answers_that_touch_the_same_place_act_in_the_order_the_seats_were_asked() {
        let mut game = build(&spec_asking(ask(json!("OTHERS"), None, false)));
        act(&mut game, "CALL");
        put(&mut game, "C", "Q", "0,0");
        put(&mut game, "B", "P", "0,0");
        assert_eq!(
            on_board(&game),
            ["0,0=CQ"],
            "C's answer is played after B's"
        );
    }

    #[test]
    fn an_open_answer_is_played_as_it_arrives_and_the_others_see_what_it_did() {
        let mut game = build(&spec_asking(ask(json!("OTHERS"), Some(false), false)));
        act(&mut game, "CALL");
        put(&mut game, "B", "P", "0,0");
        assert_eq!(on_board(&game), ["0,0=BP"]);
        assert_eq!(log(&game), ["A CALL", "B PUT"].map(String::from));
        assert_eq!(game.waiting(), ["C"]);
        game.choose_as("C", Answer::Code("Q".into())).unwrap();
        assert!(
            !places_for(&game, "C").contains(&"0,0".to_string()),
            "the square is taken, as C is shown"
        );
        game.cancel_as("C").unwrap();
        put(&mut game, "C", "Q", "1,1");
        assert_eq!(on_board(&game), ["0,0=BP", "1,1=CQ"]);
        assert_eq!(game.current_player(), "B");
    }

    #[test]
    fn part_way_answers_are_kept_for_each_seat_and_taken_back_by_its_own_cancel() {
        let mut game = build(&spec_asking(ask(json!("OTHERS"), None, false)));
        act(&mut game, "CALL");
        game.choose_as("B", Answer::Code("P".into())).unwrap();
        assert_eq!(
            game.focused("B").unwrap().choice().chosen,
            [Answer::Code("P".into())]
        );
        assert!(game.focused("C").unwrap().choice().chosen.is_empty());
        game.cancel_as("B").unwrap();
        assert!(game.focused("B").unwrap().choice().chosen.is_empty());
    }

    #[test]
    fn a_game_saved_mid_ask_resumes_with_the_same_seats_and_kept_answers() {
        for sealed in [true, false] {
            let spec = spec_asking(ask(json!("OTHERS"), Some(sealed), true));
            let mut played = build(&spec);
            act(&mut played, "CALL");
            put(&mut played, "B", "P", "0,0");
            played.choose_as("C", Answer::Code("Q".into())).unwrap();
            let saved = played.save_state().unwrap();
            let mut resumed = build(&spec);
            resumed.restore_state(&saved).unwrap();
            assert!(resumed.stage_fits_the_rules(), "sealed {sealed}");
            assert_eq!(resumed.waiting(), ["C"]);
            assert_eq!(
                resumed.focused("C").unwrap().choice().chosen,
                [Answer::Code("Q".into())]
            );
            resumed.choose_as("C", Answer::Place("1,1".into())).unwrap();
            // The asker moves again in AFTER.
            assert_eq!(resumed.current_player(), "A", "sealed {sealed}");
            assert_eq!(on_board(&resumed), ["0,0=BP", "1,1=CQ"]);
            act(&mut resumed, "FOLLOW");
            assert_eq!(resumed.current_player(), "B");
        }
    }

    #[test]
    fn the_saved_state_of_a_game_with_no_ask_has_no_trace_of_one() {
        let game = build(&spec_asking(ask(json!("OTHERS"), None, false)));
        let saved = game.save_state().unwrap();
        assert!(
            !saved.contains("together") && !saved.contains("asked"),
            "{saved}"
        );
    }

    #[test]
    fn a_state_that_waits_on_what_the_rules_lack_is_not_resumed() {
        let spec = spec_asking(ask(json!("OTHERS"), None, false));
        let mut played = build(&spec);
        act(&mut played, "CALL");
        put(&mut played, "B", "P", "0,0");
        played.choose_as("C", Answer::Code("Q".into())).unwrap();
        let saved: Value = serde_json::from_str(&played.save_state().unwrap()).unwrap();
        let fits = |change: &dyn Fn(&mut Value)| {
            let mut state = saved.clone();
            change(&mut state);
            let mut game = build(&spec);
            game.restore_state(&state.to_string()).unwrap();
            game.stage_fits_the_rules()
        };
        assert!(fits(&|_| {}), "the state as saved fits");
        assert!(!fits(
            &|state| state["together"]["seats"][0]["player"] = json!("Z")
        ));
        assert!(!fits(&|state| state["together"]["stage"] = json!("NOWHERE")));
        assert!(!fits(
            &|state| state["together"]["seats"][1]["chosen"] = json!([{ "code": "Z" }])
        ));
        assert!(!fits(&|state| {
            state["together"]["seats"][0]["locked"]["action"] = json!("NOWHERE")
        }));
        assert!(!fits(
            &|state| state["asked"] = json!([{ "player": "B", "stage": "REPLY" }])
        ));
    }

    #[test]
    fn a_seat_with_no_way_is_not_asked_and_with_none_the_turn_goes_on() {
        let mut spec = spec_asking(ask(json!("OTHERS"), None, true));
        spec["turns"]["stages"][0]["players"] = json!(["C"]);
        let mut game = build(&spec);
        act(&mut game, "CALL");
        assert_eq!(game.waiting(), ["C"]);
        spec["turns"]["stages"][0]["players"] = json!(["A"]);
        let mut game = build(&spec);
        act(&mut game, "CALL");
        assert!(!game.together_pending());
        assert_eq!(game.current_player(), "A", "the asker moves again at once");
    }

    #[test]
    fn a_seat_left_with_no_way_by_an_open_answer_is_dropped() {
        // One square, so the first answer leaves the second seat nowhere to put its piece.
        let mut spec = spec_asking(ask(json!("OTHERS"), Some(false), false));
        spec["board"]["dimensions"] = json!([1, 1]);
        let mut game = build(&spec);
        act(&mut game, "CALL");
        put(&mut game, "B", "P", "0,0");
        assert!(
            !game.together_pending(),
            "C could never answer, and is not waited on"
        );
        assert_eq!(game.current_player(), "B");
    }

    #[test]
    fn end_checks_wait_until_the_turn_is_handed_on() {
        for sealed in [true, false] {
            let mut spec = spec_asking(ask(json!("OTHERS"), Some(sealed), true));
            spec["end_checks"] = json!([{
                "when": { "test": "COUNT", "in": { "region": "BOARD" }, "min": 1 },
                "then": { "outcome": "DRAW" }
            }]);
            let mut game = build(&spec);
            act(&mut game, "CALL");
            put(&mut game, "B", "P", "0,0");
            assert!(
                matches!(game.state.phase, GamePhase::Choosing { .. }),
                "sealed {sealed}"
            );
            put(&mut game, "C", "Q", "1,1");
            assert!(
                matches!(game.state.phase, GamePhase::Choosing { .. }),
                "then, A moves"
            );
            act(&mut game, "FOLLOW");
            assert!(
                matches!(game.state.phase, GamePhase::GameOver { .. }),
                "sealed {sealed}"
            );
        }
    }

    #[test]
    fn the_asker_may_be_asked_too() {
        let mut game = build(&spec_asking(ask(json!("ALL"), None, false)));
        act(&mut game, "CALL");
        assert_eq!(game.waiting(), ["B", "C", "A"]);
        put(&mut game, "A", "P", "0,0");
        put(&mut game, "C", "P", "1,0");
        put(&mut game, "B", "P", "0,1");
        assert_eq!(on_board(&game), ["0,0=AP", "0,1=BP", "1,0=CP"]);
        assert_eq!(game.current_player(), "B");
    }

    // ---- what the spec may not say

    #[test]
    fn sealed_is_for_asks_that_ask_all_at_once() {
        let mut in_order = ask(json!("OTHERS"), Some(true), false);
        in_order["mode"] = json!("IN_ORDER");
        let error = refusal(&spec_asking(in_order));
        assert!(
            error.contains("sealed") && error.contains("TOGETHER"),
            "{error}"
        );
        let mut no_mode = ask(json!("OTHERS"), Some(false), false);
        no_mode.as_object_mut().unwrap().remove("mode");
        assert!(refusal(&spec_asking(no_mode)).contains("sealed"));
        accepted(&spec_asking(ask(json!("OTHERS"), Some(false), false)));
        accepted(&spec_asking(ask(json!("OTHERS"), Some(true), true)));
    }

    #[test]
    fn answers_given_in_any_order_do_not_roll_shuffle_stage_or_ask() {
        for effect in [
            json!({ "effect": "STAGE", "name": "AFTER" }),
            json!({ "effect": "ASK", "players": "MOVER", "stage": "REPLY" }),
            json!({ "effect": "ROLL", "at": { "bag": "DICE" }, "faces": [{ "P": 1 }] }),
            json!({ "effect": "SHUFFLE", "at": { "bag": "DICE" } }),
        ] {
            let mut spec = spec_asking(ask(json!("OTHERS"), None, false));
            spec["bags"] = json!([{ "name": "DICE", "ordered": true }]);
            spec["actions"][1]["do"]
                .as_array_mut()
                .unwrap()
                .push(effect.clone());
            let error = refusal(&spec);
            assert!(
                error.contains("PUT") && error.contains("all at once"),
                "{effect}: {error}"
            );
            // The same answer is fine when the seats answer one after another.
            let mut in_order = spec_asking(ask(json!("OTHERS"), None, false));
            in_order["actions"][0]["do"][0]["mode"] = json!("IN_ORDER");
            in_order["bags"] = spec["bags"].clone();
            in_order["actions"][1]["do"]
                .as_array_mut()
                .unwrap()
                .push(effect.clone());
            if effect["effect"] == "ASK" || effect["effect"] == "STAGE" {
                accepted(&in_order);
            }
        }
    }

    #[test]
    fn an_answer_does_not_ask_all_at_once() {
        let mut spec = spec_asking(ask(json!("OTHERS"), None, false));
        spec["actions"][0]["do"][0]["mode"] = json!("IN_ORDER");
        spec["actions"][1]["do"]
            .as_array_mut()
            .unwrap()
            .push(ask(json!("MOVER"), None, false));
        let error = refusal(&spec);
        assert!(
            error.contains("PUT") && error.contains("all at once"),
            "{error}"
        );
    }

    #[test]
    fn a_seat_whose_ways_all_start_with_one_piece_has_it_chosen_and_cannot_cancel() {
        let mut spec = spec_asking(ask(json!("OTHERS"), None, false));
        spec["players"][1]["starting_positions"] = json!([{ "piece": "P", "positions": [[0, 0]] }]);
        spec["actions"][1] = json!({ "name": "TAKE", "stage": "REPLY", "do": [
            { "choose": "PIECE", "where": { "owner": "ALLY" } },
            { "effect": "REMOVE", "at": { "place": "START" } }
        ] });
        let mut game = build(&spec);
        act(&mut game, "CALL");
        assert_eq!(game.waiting(), ["B"], "C has no piece, and so no way");
        let chosen = game.focused("B").unwrap().choice().chosen;
        assert_eq!(chosen, [Answer::Place("0,0".into())]);
        assert!(game.cancel_as("B").is_err(), "the piece stays chosen");
        game.choose_as("B", Answer::Action("TAKE".into())).unwrap();
        assert!(on_board(&game).is_empty());
    }
}
