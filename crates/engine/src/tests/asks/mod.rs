//! Answers out of turn, one after another: the `ASK` effect with `IN_ORDER`, an action that
//! belongs to a stage, and a stage for some players only. How a whole game plays with them is
//! proved by Go's dead-stone agreement, in the session crate's tests; this is what the effect
//! does and what the spec may and may not say.
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

    /// Three players, `A`, `B` and `C`, in that order. `CALL` asks as `ask` says; `ANSWER` is what
    /// an asked seat does in `REPLY`, and `FOLLOW` what the asker does in `AFTER`.
    fn spec_asking(ask: Value) -> Value {
        json!({
            "format_version": 1,
            "name": "ASKING",
            "board": { "dimensions": [2, 2] },
            "players": [
                { "name": "A", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "B", "direction": [[-1, 0], [0, -1]], "starting_positions": [] },
                { "name": "C", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["A", "B", "C"], "stages": [{ "name": "REPLY" }, { "name": "AFTER" }] },
            "pieces": [{ "code": "P", "name": "p", "moves": [] }],
            "counters": ["ANSWERS", "FOLLOWED"],
            "actions": [
                { "name": "CALL", "do": [ask] },
                { "name": "ANSWER", "stage": "REPLY",
                  "do": [{ "effect": "ADD", "counter": "ANSWERS", "to": "ALL" }] },
                { "name": "FOLLOW", "stage": "AFTER",
                  "do": [{ "effect": "ADD", "counter": "FOLLOWED" }] }
            ]
        })
    }

    fn ask(players: Value, then: bool) -> Value {
        let mut ask = json!({ "effect": "ASK", "players": players, "stage": "REPLY" });
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

    /// Who played what, in order.
    fn log(game: &Game) -> Vec<String> {
        game.history()
            .iter()
            .map(|record| format!("{} {}", record.player, record.action))
            .collect()
    }

    #[test]
    fn the_asked_seats_answer_one_after_another_and_then_the_asker_moves_again() {
        let mut game = build(&spec_asking(ask(json!("OTHERS"), true)));
        assert_eq!(game.current_player(), "A");
        act(&mut game, "CALL");
        assert_eq!(
            game.current_player(),
            "B",
            "the first asked seat is to move"
        );
        act(&mut game, "ANSWER");
        assert_eq!(game.current_player(), "C");
        act(&mut game, "ANSWER");
        // The asker moves again, in the stage `then` names, with only its actions.
        assert_eq!(game.current_player(), "A");
        assert!(game.choose(Answer::Action("CALL".into())).is_err());
        act(&mut game, "FOLLOW");
        assert_eq!(game.current_player(), "B", "then the turn is handed on");
        assert_eq!(
            log(&game),
            ["A CALL", "B ANSWER", "C ANSWER", "A FOLLOW"].map(String::from)
        );
    }

    #[test]
    fn without_then_the_turn_is_handed_on_after_the_last_answer() {
        let mut game = build(&spec_asking(ask(json!("OTHERS"), false)));
        act(&mut game, "CALL");
        act(&mut game, "ANSWER");
        act(&mut game, "ANSWER");
        assert_eq!(game.current_player(), "B", "the turn after the asker's");
    }

    /// The seats asked when `A` calls, in the order they are to move.
    fn asked_by_a(players: Value) -> Vec<String> {
        let mut game = build(&spec_asking(ask(players, false)));
        act(&mut game, "CALL");
        let mut asked = Vec::new();
        while !game.state.asked.is_empty() {
            asked.push(game.current_player());
            act(&mut game, "ANSWER");
        }
        asked
    }

    #[test]
    fn who_is_asked_and_in_what_order() {
        assert_eq!(asked_by_a(json!("OTHERS")), ["B", "C"]);
        assert_eq!(asked_by_a(json!("ALL")), ["B", "C", "A"]);
        assert_eq!(asked_by_a(json!("MOVER")), ["A"]);
        assert_eq!(asked_by_a(json!(["C", "B"])), ["C", "B"]);
    }

    #[test]
    fn the_order_starts_after_the_asker() {
        // B asks: C, then A.
        let mut game = build(&spec_asking(ask(json!("OTHERS"), false)));
        act(&mut game, "CALL");
        act(&mut game, "ANSWER");
        act(&mut game, "ANSWER");
        assert_eq!(game.current_player(), "B");
        act(&mut game, "CALL");
        assert_eq!(game.current_player(), "C");
        act(&mut game, "ANSWER");
        assert_eq!(game.current_player(), "A");
    }

    #[test]
    fn a_seat_with_no_way_in_the_stage_is_skipped() {
        let mut spec = spec_asking(ask(json!("OTHERS"), true));
        spec["turns"]["stages"][0]["players"] = json!(["C"]);
        let mut game = build(&spec);
        act(&mut game, "CALL");
        assert_eq!(game.current_player(), "C", "B has no way in REPLY");
        act(&mut game, "ANSWER");
        assert_eq!(game.current_player(), "A");
        assert_eq!(log(&game), ["A CALL", "C ANSWER"].map(String::from));
    }

    #[test]
    fn when_every_seat_is_skipped_the_asker_goes_on_at_once() {
        let mut spec = spec_asking(ask(json!("OTHERS"), true));
        spec["turns"]["stages"][0]["players"] = json!(["A"]);
        let mut game = build(&spec);
        act(&mut game, "CALL");
        assert_eq!(game.current_player(), "A");
        act(&mut game, "FOLLOW");
        assert_eq!(game.current_player(), "B");
        // And with no `then`, the turn is handed on.
        let mut spec = spec_asking(ask(json!("OTHERS"), false));
        spec["turns"]["stages"][0]["players"] = json!(["A"]);
        let mut game = build(&spec);
        act(&mut game, "CALL");
        assert_eq!(game.current_player(), "B");
    }

    #[test]
    fn an_answer_may_ask_again_and_is_answered_before_the_seats_waiting() {
        // Each seat answers twice: the first answer asks the same seat again.
        let mut spec = spec_asking(ask(json!("OTHERS"), true));
        spec["actions"][1]["do"] = json!([
            { "effect": "ADD", "counter": "ANSWERS" },
            { "if": { "test": "COUNTER", "name": "ANSWERS", "max": 0 },
              "then": [{ "effect": "ASK", "players": "MOVER", "stage": "REPLY" }] }
        ]);
        let mut game = build(&spec);
        act(&mut game, "CALL");
        let mut asked = Vec::new();
        while game.current_player() != "A" {
            asked.push(game.current_player());
            act(&mut game, "ANSWER");
        }
        assert_eq!(asked, ["B", "B", "C", "C"]);
        // The first ask's `then` still holds.
        act(&mut game, "FOLLOW");
        assert_eq!(game.current_player(), "B");
    }

    #[test]
    fn end_checks_wait_until_the_turn_is_handed_on() {
        let mut spec = spec_asking(ask(json!("OTHERS"), true));
        spec["end_checks"] = json!([
            { "when": { "test": "COUNTER", "name": "ANSWERS", "min": 1 },
              "then": { "outcome": "DRAW" } }
        ]);
        let mut game = build(&spec);
        act(&mut game, "CALL");
        act(&mut game, "ANSWER");
        act(&mut game, "ANSWER");
        assert!(
            matches!(game.state.phase, GamePhase::Choosing { .. }),
            "the answers are part of the asker's turn"
        );
        act(&mut game, "FOLLOW");
        assert!(matches!(game.state.phase, GamePhase::GameOver { .. }));
    }

    #[test]
    fn a_game_saved_mid_ask_resumes_with_the_same_seat_to_move() {
        let mut played = build(&spec_asking(ask(json!("OTHERS"), true)));
        let saved_before = played.save_state().unwrap();
        assert!(
            !saved_before.contains("asked"),
            "a game with no ask in progress saves as it always has: {saved_before}"
        );
        act(&mut played, "CALL");
        act(&mut played, "ANSWER");
        let saved = played.save_state().unwrap();
        let mut resumed = build(&spec_asking(ask(json!("OTHERS"), true)));
        resumed.restore_state(&saved).unwrap();
        assert_eq!(resumed.current_player(), "C");
        act(&mut resumed, "ANSWER");
        assert_eq!(resumed.current_player(), "A");
        act(&mut resumed, "FOLLOW");
        assert_eq!(resumed.current_player(), "B");
        assert!(resumed.stage_fits_the_rules());
    }

    #[test]
    fn a_game_without_an_ask_plays_as_before() {
        // An action without a stage is offered in a stage no action belongs to.
        let mut spec = spec_asking(ask(json!("OTHERS"), true));
        spec["actions"] = json!([
            { "name": "CALL", "do": [{ "effect": "STAGE", "name": "AFTER" }] },
            { "name": "MORE", "do": [] }
        ]);
        let mut game = build(&spec);
        act(&mut game, "CALL");
        assert_eq!(game.current_player(), "A", "STAGE keeps the turn");
        act(&mut game, "MORE");
        assert_eq!(game.current_player(), "B");
    }

    // ---- what the spec may not say

    #[test]
    fn an_ask_names_stages_and_players_the_game_has() {
        let unknown_stage = refusal(&spec_asking(
            json!({ "effect": "ASK", "players": "OTHERS", "stage": "NOWHERE" }),
        ));
        assert!(
            unknown_stage.contains("NOWHERE") && unknown_stage.contains("turns.stages"),
            "{unknown_stage}"
        );
        let unknown_then = refusal(&spec_asking(json!({ "effect": "ASK", "players": "OTHERS",
            "stage": "REPLY", "then": { "stage": "NOWHERE" } })));
        assert!(unknown_then.contains("NOWHERE"), "{unknown_then}");
        let stranger = refusal(&spec_asking(ask(json!(["B", "Z"]), false)));
        assert!(
            stranger.contains('Z') && stranger.contains("not a player"),
            "{stranger}"
        );
        let twice = refusal(&spec_asking(ask(json!(["B", "B"]), false)));
        assert!(twice.contains("twice"), "{twice}");
        let empty = refusal(&spec_asking(ask(json!([]), false)));
        assert!(empty.contains("empty"), "{empty}");
        let word = refusal(&spec_asking(ask(json!("EVERYONE"), false)));
        assert!(
            word.contains("EVERYONE") && word.contains("OTHERS"),
            "{word}"
        );
    }

    #[test]
    fn an_ask_cannot_be_in_a_pieces_move() {
        let mut spec = spec_asking(ask(json!("OTHERS"), false));
        spec["pieces"][0]["moves"] = json!([{ "id": 0, "step": [0, 1],
            "do": [ask(json!("OTHERS"), false)] }]);
        let error = refusal(&spec);
        assert!(error.contains("ASK") && error.contains("move"), "{error}");
    }

    #[test]
    fn a_way_does_not_run_both_stage_and_ask() {
        let mut spec = spec_asking(ask(json!("OTHERS"), false));
        spec["actions"][0]["do"] = json!([
            { "effect": "STAGE", "name": "AFTER" },
            ask(json!("OTHERS"), false)
        ]);
        let error = refusal(&spec);
        assert!(error.contains("STAGE") && error.contains("ASK"), "{error}");
        // In an `if` and its `else` they are two ways, not one.
        spec["actions"][0]["do"] = json!([{
            "if": { "test": "COUNTER", "name": "ANSWERS", "max": 0 },
            "then": [{ "effect": "STAGE", "name": "AFTER" }],
            "else": [ask(json!("OTHERS"), false)]
        }]);
        accepted(&spec);
        // And one in the action, the other in an `every_action` entry, is one way.
        spec["actions"][0]["do"] = json!([{ "effect": "STAGE", "name": "AFTER" }]);
        spec["every_action"] = json!([{ "for": ["CALL"], "do": [ask(json!("OTHERS"), false)] }]);
        assert!(refusal(&spec).contains("STAGE"));
    }

    #[test]
    fn a_stage_that_is_asked_in_has_an_action_that_belongs_to_it() {
        let mut spec = spec_asking(ask(json!("OTHERS"), false));
        spec["actions"][1]["stage"] = json!("AFTER");
        let error = refusal(&spec);
        assert!(
            error.contains("REPLY") && error.contains("no action"),
            "{error}"
        );
    }

    #[test]
    fn an_action_belongs_to_a_declared_stage() {
        let mut spec = spec_asking(ask(json!("OTHERS"), false));
        spec["actions"][2]["stage"] = json!("NOWHERE");
        let error = refusal(&spec);
        assert!(
            error.contains("NOWHERE") && error.contains("turns.stages"),
            "{error}"
        );
    }

    #[test]
    fn an_answer_that_asks_again_has_no_then_of_its_own() {
        let mut spec = spec_asking(ask(json!("OTHERS"), true));
        spec["actions"][1]["do"] = json!([ask(json!("MOVER"), true)]);
        let error = refusal(&spec);
        assert!(
            error.contains("ANSWER") && error.contains("then"),
            "{error}"
        );
        spec["actions"][1]["do"] = json!([ask(json!("MOVER"), false)]);
        accepted(&spec);
    }

    #[test]
    fn a_stage_is_for_players_the_game_has() {
        let mut spec = spec_asking(ask(json!("OTHERS"), false));
        spec["turns"]["stages"][0]["players"] = json!([]);
        assert!(refusal(&spec).contains("empty"));
        spec["turns"]["stages"][0]["players"] = json!(["Z"]);
        let error = refusal(&spec);
        assert!(
            error.contains('Z') && error.contains("not a player"),
            "{error}"
        );
    }

    #[test]
    fn the_mode_is_in_order_or_together() {
        let mut in_order = ask(json!("OTHERS"), false);
        in_order["mode"] = json!("IN_ORDER");
        accepted(&spec_asking(in_order));
        let mut nonsense = ask(json!("OTHERS"), false);
        nonsense["mode"] = json!("BACKWARDS");
        assert!(read(&spec_asking(nonsense)).is_err());
    }

    #[test]
    fn a_state_that_waits_on_a_seat_the_rules_lack_is_not_resumed() {
        let spec = spec_asking(ask(json!("OTHERS"), true));
        let mut played = build(&spec);
        act(&mut played, "CALL");
        let saved: Value = serde_json::from_str(&played.save_state().unwrap()).unwrap();
        let fits = |changed: &dyn Fn(&mut Value)| {
            let mut state = saved.clone();
            changed(&mut state);
            let mut game = build(&spec);
            game.restore_state(&state.to_string()).unwrap();
            game.stage_fits_the_rules()
        };
        assert!(fits(&|_| {}), "the state as saved fits");
        assert!(!fits(&|state| state["asked"][0]["player"] = json!("Z")));
        assert!(!fits(&|state| state["asked"][0]["stage"] = json!("NOWHERE")));
        assert!(!fits(&|state| state["ask_then"] = json!("NOWHERE")));
        // Seats are waited on while the turn is in some other stage.
        assert!(!fits(&|state| state["stage"]["name"] = json!("AFTER")));
        assert!(!fits(&|state| {
            state.as_object_mut().unwrap().remove("stage");
        }));
        // A `then` with no one asked is a state no rules save.
        assert!(!fits(&|state| {
            state["asked"] = json!([]);
        }));
    }
}
