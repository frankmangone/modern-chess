//! Counters: the effects `ADD` and `RESET`, applied when a way is played, and the state that
//! keeps them.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Game, Piece};
    use crate::shared::Position;
    use crate::specs::{parse_game_spec_str, GameSpecError};
    use crate::tests::support::{go, pick};

    const WHITE: &str = "WHITE";
    const BLACK: &str = "BLACK";

    fn at(place: &str) -> Value {
        json!({ "place": place })
    }

    /// A 4×4 game with a PROBE for each player that steps one square along its own file, and
    /// the counter `X`. The action `A` chooses a piece and a move, then `after`.
    fn spec(after: Vec<Value>) -> Value {
        let mut steps = vec![
            json!({ "choose": "PIECE", "where": { "owner": "ALLY" } }),
            json!({ "choose": "MOVE" }),
        ];
        steps.extend(after);
        json!({
            "name": "COUNTERS",
            "board": { "dimensions": [4, 4] },
            "counters": ["X"],
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "pieces": [{ "code": "PROBE", "name": "probe", "moves": [{
                "id": 0, "step": [0, 1],
                "do": [
                    { "test": "NOT", "of": { "test": "COUNT",
                        "in": { "region": "AT", "at": at("TARGET") },
                        "where": { "owner": "ALLY" } } },
                    { "effect": "MOVE", "from": at("START"), "to": at("TARGET") }
                ]
            }] }],
            "actions": [{ "name": "A", "do": steps }]
        })
    }

    fn game_of(spec: &Value) -> Game {
        let spec = parse_game_spec_str(&spec.to_string()).expect("the spec is read");
        let mut game = Game::from_spec(spec);
        game.state
            .pieces
            .insert(pos![0, 0], Piece::new("PROBE".into(), WHITE.into()));
        game.state
            .pieces
            .insert(pos![3, 3], Piece::new("PROBE".into(), BLACK.into()));
        game.work_out_ways();
        game
    }

    fn white_moves(game: &mut Game) {
        pick(game, pos![0, 0]).unwrap();
        go(game, pos![0, 1]).unwrap();
    }

    fn add(extra: Value) -> Value {
        let mut effect = json!({ "effect": "ADD", "counter": "X" });
        for (key, value) in extra.as_object().cloned().unwrap_or_default() {
            effect[key] = value;
        }
        effect
    }

    #[test]
    fn add_changes_the_movers_counter_when_the_way_is_played() {
        let mut game = game_of(&spec(vec![add(json!({ "by": 2 }))]));
        white_moves(&mut game);
        assert_eq!(game.counter(WHITE, "X"), 2);
        assert_eq!(game.counter(BLACK, "X"), 0);
    }

    #[test]
    fn add_without_by_adds_one() {
        let mut game = game_of(&spec(vec![add(json!({}))]));
        white_moves(&mut game);
        assert_eq!(game.counter(WHITE, "X"), 1);
    }

    #[test]
    fn add_to_all_changes_every_players_counter() {
        let mut game = game_of(&spec(vec![add(json!({ "to": "ALL" }))]));
        white_moves(&mut game);
        assert_eq!((game.counter(WHITE, "X"), game.counter(BLACK, "X")), (1, 1));
    }

    #[test]
    fn reset_sets_the_counter_to_zero_for_the_mover_or_for_all() {
        let reset = |to: Value| {
            let mut effect = json!({ "effect": "RESET", "counter": "X" });
            if !to.is_null() {
                effect["to"] = to;
            }
            effect
        };
        let mut game = game_of(&spec(vec![reset(Value::Null)]));
        game.state
            .counters
            .insert(WHITE.into(), [("X".into(), 5)].into());
        game.state
            .counters
            .insert(BLACK.into(), [("X".into(), 7)].into());
        white_moves(&mut game);
        assert_eq!((game.counter(WHITE, "X"), game.counter(BLACK, "X")), (0, 7));

        let mut game = game_of(&spec(vec![reset(json!("ALL"))]));
        game.state
            .counters
            .insert(WHITE.into(), [("X".into(), 5)].into());
        game.state
            .counters
            .insert(BLACK.into(), [("X".into(), 7)].into());
        white_moves(&mut game);
        assert_eq!((game.counter(WHITE, "X"), game.counter(BLACK, "X")), (0, 0));
    }

    #[test]
    fn a_way_that_is_not_played_changes_nothing() {
        let mut game = game_of(&spec(vec![add(json!({}))]));
        pick(&mut game, pos![0, 0]).unwrap();
        assert_eq!(game.counter(WHITE, "X"), 0, "picked, not yet played");
        assert!(game.state.counters.is_empty());
    }

    #[test]
    fn a_counter_test_in_a_way_reads_the_counter_as_the_turn_began() {
        // Adds one, then requires that the counter is still 0: it reads the turn's start.
        let steps = vec![
            add(json!({})),
            json!({ "test": "COUNTER", "name": "X", "max": 0 }),
        ];
        let mut game = game_of(&spec(steps));
        assert_eq!(
            game.compute_ways().len(),
            1,
            "the counter is 0 as the turn begins"
        );
        white_moves(&mut game);
        pick(&mut game, pos![3, 3]).unwrap();
        go(&mut game, pos![3, 2]).unwrap();
        assert!(
            game.compute_ways().is_empty(),
            "WHITE's counter is 1 now, so the test fails at the start of its turn"
        );
    }

    #[test]
    fn counters_survive_a_save_and_restore_and_a_state_without_them_restores_with_zeros() {
        let mut game = game_of(&spec(vec![add(json!({ "by": 3 }))]));
        white_moves(&mut game);
        let saved = game.save_state().unwrap();
        let mut restored = game_of(&spec(vec![add(json!({ "by": 3 }))]));
        restored.restore_state(&saved).unwrap();
        assert_eq!(restored.counter(WHITE, "X"), 3);

        let mut old: Value = serde_json::from_str(&saved).unwrap();
        old.as_object_mut().unwrap().remove("counters");
        let mut zeros = game_of(&spec(vec![add(json!({ "by": 3 }))]));
        zeros.restore_state(&old.to_string()).unwrap();
        assert_eq!(zeros.counter(WHITE, "X"), 0);
    }

    fn refused(spec: Value) -> String {
        match parse_game_spec_str(&spec.to_string()) {
            Err(GameSpecError::InvalidStep(message)) => message,
            other => panic!("expected an invalid step, got {other:?}"),
        }
    }

    #[test]
    fn an_effect_on_a_counter_that_is_not_declared_is_refused() {
        let message = refused(spec(vec![json!({ "effect": "ADD", "counter": "Y" })]));
        assert!(
            message.contains("`Y`") && message.contains("counters"),
            "{message}"
        );
        let message = refused(spec(vec![json!({ "effect": "RESET", "counter": "Y" })]));
        assert!(message.contains("`Y`"), "{message}");
    }

    #[test]
    fn adding_zero_or_to_an_unknown_player_is_refused() {
        let message = refused(spec(vec![add(json!({ "by": 0 }))]));
        assert!(message.contains("adds 0"), "{message}");
        let message = refused(spec(vec![add(json!({ "to": "NEXT" }))]));
        assert!(
            message.contains("NEXT") && message.contains("ALL"),
            "{message}"
        );
    }

    #[test]
    fn a_counter_effect_may_sit_inside_a_for_each() {
        let steps = vec![json!({ "effect": "FOR_EACH",
            "in": { "region": "AROUND", "of": at("TARGET") },
            "do": [add(json!({}))] })];
        let mut game = game_of(&spec(steps));
        white_moves(&mut game);
        assert!(game.counter(WHITE, "X") >= 2, "one per neighbour of 0,1");
        let _: Position = pos![0, 0];
    }
}
