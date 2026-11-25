//! Chance: a seed that lives with the match, the `ROLL` effect, ordered bags (piles) and the
//! `SHUFFLE` effect. A small deck game written for these tests plays them; the dice race has its
//! own file.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::places::holders::bag_position;
    use crate::logic::{Answer, Game, GameTransition, MoveRecord};
    use crate::shared::Symbol;
    use crate::specs::{parse_game_spec_str, GameSpecError};

    fn read(spec: &Value) -> Result<crate::specs::GameSpec, GameSpecError> {
        parse_game_spec_str(&spec.to_string())
    }

    fn game(spec: &Value) -> Game {
        Game::from_spec(read(spec).expect("the spec reads"))
    }

    fn refusal(spec: &Value) -> String {
        read(spec).expect_err("the spec is refused").to_string()
    }

    /// A deck game: a pile of six cards (three A, two B, one C, started in that order from the
    /// top), each player's hand, and a shared tray. The actions are given.
    fn deck_spec(actions: Value) -> Value {
        json!({
            "format_version": 1,
            "name": "DECK",
            "board": { "dimensions": [2, 2] },
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["WHITE", "BLACK"], "stages": [{ "name": "AFTER" }] },
            "pieces": [
                { "code": "A", "name": "a", "moves": [] },
                { "code": "B", "name": "b", "moves": [] },
                { "code": "C", "name": "c", "moves": [] },
                { "code": "PIP", "name": "pip", "moves": [] }
            ],
            "bags": [
                { "name": "DECK", "ordered": true, "start": { "A": 3, "B": 2, "C": 1 } },
                { "name": "DISCARD", "ordered": true },
                { "name": "HAND", "for": "EACH_PLAYER" },
                { "name": "TRAY" }
            ],
            "actions": actions
        })
    }

    fn deck() -> Value {
        json!({ "bag": "DECK" })
    }

    fn hand() -> Value {
        json!({ "bag": "HAND", "of": "MOVER" })
    }

    fn shuffle_and_draw(cards: u32) -> Value {
        json!([{ "name": "DRAW", "do": [
            { "effect": "SHUFFLE", "at": deck() },
            { "effect": "MOVE", "from": deck(), "to": hand(), "count": cards }
        ] }])
    }

    fn die() -> Value {
        json!([{ "PIP": 1 }, { "PIP": 2 }, { "PIP": 3 }, { "PIP": 4 }, { "PIP": 5 }, { "PIP": 6 }])
    }

    fn roll_action() -> Value {
        json!([{ "name": "ROLL", "do": [
            { "effect": "ROLL", "at": { "bag": "TRAY" }, "dice": 2, "faces": die() }
        ] }])
    }

    fn take(game: &mut Game, action: &str) {
        game.transition(GameTransition::Choose(Answer::Action(action.to_string())))
            .unwrap_or_else(|error| panic!("{action} is refused: {error:?}"));
    }

    /// The pieces of the bag, top first, as `A A B`.
    fn pile(game: &Game, name: &str) -> String {
        game.state
            .holders
            .get(&bag_position(name, None))
            .map(|held| {
                held.entries()
                    .iter()
                    .flat_map(|run| std::iter::repeat_n(run.code.to_string(), run.count as usize))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default()
    }

    fn pips(game: &Game) -> u32 {
        game.state
            .holders
            .get(&bag_position("TRAY", None))
            .map_or(0, |held| held.count_of("PIP"))
    }

    // ---- the seed

    #[test]
    fn a_game_that_never_rolls_keeps_no_seed() {
        let mut spec = deck_spec(json!([{ "name": "WAIT", "do": [] }]));
        spec["bags"] = json!([{ "name": "TRAY" }]);
        let mut plain = game(&spec);
        assert!(!plain.rules.chance);
        plain.set_seed(42);
        assert_eq!(plain.state.seed, None);
        let saved = plain.save_state().expect("saves");
        assert!(!saved.contains("seed") && !saved.contains("draws"), "{saved}");
    }

    #[test]
    fn a_game_that_rolls_keeps_its_seed_and_how_many_numbers_it_used() {
        let mut rolling = game(&deck_spec(roll_action()));
        assert!(rolling.rules.chance);
        rolling.set_seed(42);
        assert_eq!(rolling.state.seed, Some(42));
        take(&mut rolling, "ROLL");
        assert_eq!(rolling.state.draws, 2, "two dice used two numbers");
        let saved: Value = serde_json::from_str(&rolling.save_state().expect("saves")).expect("json");
        assert_eq!(saved["seed"], 42);
        assert_eq!(saved["draws"], 2);

        let mut resumed = game(&deck_spec(roll_action()));
        resumed
            .restore_state(&saved.to_string())
            .expect("a saved state loads");
        assert_eq!(resumed.state.seed, Some(42));
        assert_eq!(resumed.state.draws, 2);
    }

    #[test]
    fn a_state_saved_before_chance_existed_loads_without_a_seed() {
        let mut plain = game(&deck_spec(roll_action()));
        let mut saved: Value =
            serde_json::from_str(&plain.save_state().expect("saves")).expect("json");
        saved.as_object_mut().expect("object").remove("seed");
        saved.as_object_mut().expect("object").remove("draws");
        plain.restore_state(&saved.to_string()).expect("loads");
        assert_eq!(plain.state.seed, None);
        assert_eq!(plain.state.draws, 0);
    }

    #[test]
    fn the_same_seed_rolls_the_same_dice_and_another_seed_does_not() {
        let rolled = |seed: u64| {
            let mut rolling = game(&deck_spec(roll_action()));
            rolling.set_seed(seed);
            take(&mut rolling, "ROLL");
            pips(&rolling)
        };
        assert_eq!(rolled(7), rolled(7));
        let all: std::collections::BTreeSet<u32> = (0..40).map(rolled).collect();
        assert!(all.len() > 3, "40 seeds gave {all:?}");
        assert!(all.iter().all(|total| (2..=12).contains(total)));
    }

    #[test]
    fn working_the_ways_out_or_cancelling_does_not_use_a_number() {
        let mut rolling = game(&deck_spec(roll_action()));
        rolling.set_seed(9);
        rolling.work_out_ways();
        rolling.work_out_ways();
        assert_eq!(rolling.state.draws, 0);
        let ways = rolling.state.ways.as_ref().expect("ways");
        assert_eq!(ways.len(), 1);
        assert_eq!(ways.first().map(|way| way.draws), Some(2));
    }

    #[test]
    fn a_roll_is_the_same_whether_the_turn_was_worked_out_before_or_after_the_seed() {
        let mut late = game(&deck_spec(roll_action()));
        late.set_seed(5);
        let mut early = game(&deck_spec(roll_action()));
        early.state.seed = Some(5);
        early.work_out_ways();
        take(&mut late, "ROLL");
        take(&mut early, "ROLL");
        assert_eq!(pips(&late), pips(&early));
    }

    // ---- ROLL

    #[test]
    fn dice_put_the_pieces_of_their_faces_in_the_bag() {
        let mut spec = deck_spec(json!([{ "name": "ROLL", "do": [
            { "effect": "ROLL", "at": { "bag": "TRAY" }, "dice": 30,
              "faces": [{ "A": 2 }, { "B": 1, "C": 1 }] }
        ] }]));
        spec["bags"][3]["capacity"] = json!(100);
        let mut rolling = game(&spec);
        rolling.set_seed(3);
        take(&mut rolling, "ROLL");
        let tray = rolling
            .state
            .holders
            .get(&bag_position("TRAY", None))
            .cloned()
            .expect("something was rolled");
        // Each die is either 2 A or one B and one C: B and C come up the same number of times.
        assert_eq!(tray.count_of("B"), tray.count_of("C"));
        assert_eq!(tray.count_of("A") + 2 * tray.count_of("B"), 60);
        assert!(tray.count_of("A") > 0 && tray.count_of("B") > 0);
        let record = rolling.history().last().expect("recorded");
        assert_eq!(record.rolled.len(), 30);
        assert!(record.rolled.iter().all(|die| die.bag == "TRAY"));
    }

    #[test]
    fn a_die_the_player_could_not_peek_at_has_its_face_in_the_history() {
        let mut rolling = game(&deck_spec(roll_action()));
        rolling.set_seed(11);
        take(&mut rolling, "ROLL");
        let record = rolling.history().last().expect("recorded");
        assert_eq!(record.rolled.len(), 2);
        let total: u32 = record
            .rolled
            .iter()
            .map(|die| die.face.get("PIP").copied().unwrap_or(0))
            .sum();
        assert_eq!(total, pips(&rolling));
        assert!(record.shuffled.is_empty());
    }

    #[test]
    fn the_dice_of_a_players_bag_are_that_players() {
        let mut spec = deck_spec(json!([{ "name": "ROLL", "do": [
            { "effect": "ROLL", "at": hand(), "faces": [{ "A": 1 }] }
        ] }]));
        spec["pieces"][0]["moves"] = json!([]);
        let mut rolling = game(&spec);
        rolling.set_seed(1);
        take(&mut rolling, "ROLL");
        let held = rolling
            .state
            .holders
            .get(&bag_position("HAND", Some("WHITE")))
            .expect("WHITE rolled into its own hand");
        assert_eq!(held.entries().first().map(|run| run.owner), Some(Symbol::new("WHITE")));
        assert_eq!(rolling.history().last().map(|r| r.rolled[0].bag.clone()), Some("HAND@WHITE".to_string()));
    }

    #[test]
    fn a_roll_that_does_not_fit_the_bag_is_not_offered() {
        let mut spec = deck_spec(roll_action());
        spec["bags"][3]["capacity"] = json!(1);
        let mut rolling = game(&spec);
        rolling.set_seed(1);
        assert!(rolling.state.ways.as_ref().expect("ways").is_empty());
    }

    #[test]
    fn the_number_of_dice_can_be_counted() {
        let mut spec = deck_spec(json!([{ "name": "ROLL", "do": [
            { "effect": "ROLL", "at": { "bag": "TRAY" },
              "dice": { "count": { "in": { "region": "AT", "at": { "bag": "DECK" } },
                                    "where": { "codes": ["A"] } } },
              "faces": [{ "PIP": 1 }] }
        ] }]));
        spec["bags"][3]["capacity"] = json!(100);
        let mut rolling = game(&spec);
        take(&mut rolling, "ROLL");
        assert_eq!(pips(&rolling), 3, "one die for each A in the deck");
    }

    #[test]
    fn a_faulty_roll_is_refused_when_the_spec_is_read() {
        let roll = |edit: &dyn Fn(&mut Value)| {
            let mut effect = json!({ "effect": "ROLL", "at": { "bag": "TRAY" }, "faces": die() });
            edit(&mut effect);
            deck_spec(json!([{ "name": "ROLL", "do": [effect] }]))
        };
        let cases: [(&str, &dyn Fn(&mut Value), &str); 8] = [
            ("a bag that is not declared", &|e| e["at"] = json!({ "bag": "NOPE" }), "NOPE"),
            ("a place instead of a bag", &|e| e["at"] = json!({ "place": "START" }), "at"),
            ("no faces", &|e| e["faces"] = json!([]), "faces"),
            ("an empty face", &|e| e["faces"] = json!([{}]), "face"),
            ("a face of 0 pieces", &|e| e["faces"] = json!([{ "PIP": 0 }]), "count of 0"),
            ("an unknown code", &|e| e["faces"] = json!([{ "ZZZ": 1 }]), "ZZZ"),
            ("ALL dice", &|e| e["dice"] = json!("ALL"), "ALL"),
            ("a key it does not know", &|e| e["sides"] = json!(6), "sides"),
        ];
        for (what, edit, mention) in cases {
            let message = refusal(&roll(edit));
            assert!(message.contains(mention), "{what}: {message}");
        }
        let many = json!(vec![json!({ "PIP": 1 }); 1001]);
        let message = refusal(&roll(&|e| e["faces"] = many.clone()));
        assert!(message.contains("faces"), "{message}");
    }

    #[test]
    fn chance_has_no_place_in_a_pieces_move() {
        let mut spec = deck_spec(json!([{ "name": "WAIT", "do": [] }]));
        spec["pieces"][0]["moves"] = json!([{ "id": 0, "step": [0, 1], "do": [
            { "effect": "ROLL", "at": { "bag": "TRAY" }, "faces": die() }
        ] }]);
        let message = refusal(&spec);
        assert!(message.contains("piece's move"), "{message}");
    }

    // ---- no peeking

    #[test]
    fn a_question_after_a_roll_is_refused() {
        let ask = json!({ "choose": "CODE", "as": "pick", "options": ["A", "B"] });
        let roll = json!({ "effect": "ROLL", "at": { "bag": "TRAY" }, "faces": die() });
        let shuffle = json!({ "effect": "SHUFFLE", "at": deck() });
        let plain = [
            json!([roll, ask]),
            json!([shuffle, ask]),
            json!([{ "if": { "test": "COUNT", "in": { "region": "BOARD" } },
                     "then": [roll], "else": [] }, ask]),
            json!([{ "effect": "FOR_EACH", "in": { "region": "BOARD" }, "do": [roll] }, ask]),
        ];
        for steps in plain {
            let message = refusal(&deck_spec(json!([{ "name": "GO", "do": steps }])));
            assert!(
                message.contains("asks the player a question after a ROLL or a SHUFFLE"),
                "{message}"
            );
        }
    }

    #[test]
    fn a_test_after_a_roll_is_refused_because_a_way_that_fails_is_not_offered() {
        let roll = json!({ "effect": "ROLL", "at": { "bag": "TRAY" }, "faces": die() });
        let check = json!({ "test": "COUNT", "in": { "region": "AT", "at": { "bag": "TRAY" } } });
        for steps in [json!([roll, check]), json!([{ "effect": "SHUFFLE", "at": deck() }, check])] {
            let message = refusal(&deck_spec(json!([{ "name": "GO", "do": steps }])));
            assert!(
                message.contains("has a test after a ROLL or a SHUFFLE"),
                "{message}"
            );
        }
        // A test in an `if` removes no way, so it may follow a roll.
        let ok = deck_spec(json!([{ "name": "GO", "do": [roll, { "if": check, "then": [], "else": [] }] }]));
        read(&ok).expect("reads");
    }

    #[test]
    fn questions_before_a_roll_and_a_roll_that_hands_on_by_a_stage_are_fine() {
        let ask = json!({ "choose": "CODE", "as": "pick", "options": ["A", "B"] });
        let roll = json!({ "effect": "ROLL", "at": { "bag": "TRAY" }, "faces": die() });
        let ok = deck_spec(json!([
            { "name": "ASK_THEN_ROLL", "do": [ask, roll] },
            { "name": "ROLL_THEN_STAGE", "do": [roll, { "effect": "STAGE", "name": "AFTER" }] },
            { "name": "CHOOSE_IN_THE_STAGE", "do": [ask] }
        ]));
        read(&ok).expect("reads");
    }

    // ---- piles

    #[test]
    fn a_pile_starts_in_code_order_with_the_first_code_on_top() {
        let pile_game = game(&deck_spec(json!([{ "name": "WAIT", "do": [] }])));
        assert_eq!(pile(&pile_game, "DECK"), "A A A B B C");
    }

    #[test]
    fn drawing_takes_from_the_top_and_a_piece_that_enters_goes_on_top() {
        let mut pile_game = game(&deck_spec(json!([{ "name": "DRAW", "do": [
            { "effect": "MOVE", "from": deck(), "to": hand(), "count": 4 },
            { "effect": "MOVE", "from": hand(), "to": { "bag": "DISCARD" }, "what": { "codes": ["B"] }, "count": 1 }
        ] }])));
        take(&mut pile_game, "DRAW");
        assert_eq!(pile(&pile_game, "DECK"), "B C");
        let hand_of = pile_game
            .state
            .holders
            .get(&bag_position("HAND", Some("WHITE")))
            .expect("a hand");
        assert_eq!(hand_of.count_of("A"), 3);
        assert_eq!(hand_of.count_of("B"), 0, "the B went to the discard pile");
        assert_eq!(pile(&pile_game, "DISCARD"), "B");
    }

    #[test]
    fn a_pile_moved_whole_onto_another_keeps_its_order() {
        let mut pile_game = game(&deck_spec(json!([
            { "name": "AWAY", "do": [
                { "effect": "MOVE", "from": deck(), "to": { "bag": "DISCARD" }, "count": 4 }
            ] },
            { "name": "BACK", "do": [
                { "effect": "MOVE", "from": { "bag": "DISCARD" }, "to": deck(), "count": "ALL" }
            ] }
        ])));
        take(&mut pile_game, "AWAY");
        assert_eq!(pile(&pile_game, "DISCARD"), "A A A B");
        assert_eq!(pile(&pile_game, "DECK"), "B C");
        take(&mut pile_game, "BACK");
        assert_eq!(pile(&pile_game, "DECK"), "A A A B B C", "back as it was");
        assert_eq!(pile(&pile_game, "DISCARD"), "");
    }

    #[test]
    fn drawing_more_than_the_pile_has_is_not_offered() {
        let drawing = game(&deck_spec(json!([{ "name": "DRAW", "do": [
            { "effect": "MOVE", "from": deck(), "to": hand(), "count": 7 }
        ] }])));
        assert!(drawing.state.ways.as_ref().expect("ways").is_empty());
    }

    #[test]
    fn a_faulty_pile_is_refused_when_the_spec_is_read() {
        let with = |bags: Value, actions: Value| {
            let mut spec = deck_spec(actions);
            spec["bags"] = bags;
            spec
        };
        let wait = json!([{ "name": "WAIT", "do": [] }]);
        let message = refusal(&with(json!([{ "name": "HAND", "for": "EACH_PLAYER", "ordered": true }]), wait.clone()));
        assert!(message.contains("hand"), "{message}");

        let unordered = with(json!([{ "name": "DECK" }]), json!([{ "name": "S", "do": [
            { "effect": "SHUFFLE", "at": { "bag": "DECK" } }] }]));
        let message = refusal(&unordered);
        assert!(message.contains("not ordered"), "{message}");

        let undeclared = with(json!([{ "name": "DECK", "ordered": true }]), json!([{ "name": "S", "do": [
            { "effect": "SHUFFLE", "at": { "bag": "OTHER" } }] }]));
        assert!(refusal(&undeclared).contains("OTHER"));

        let filtered = deck_spec(json!([{ "name": "D", "do": [
            { "effect": "MOVE", "from": deck(), "to": hand(), "what": { "codes": ["A"] } }] }]));
        let message = refusal(&filtered);
        assert!(message.contains("`what`"), "{message}");

        let picked = deck_spec(json!([{ "name": "D", "do": [
            { "choose": "HELD", "in": deck() }] }]));
        let message = refusal(&picked);
        assert!(message.contains("HELD") && message.contains("ordered"), "{message}");
    }

    #[test]
    fn a_pile_as_written_back_keeps_ordered_and_an_unordered_bag_does_not_grow_a_key() {
        let spec = read(&deck_spec(json!([{ "name": "WAIT", "do": [] }]))).expect("reads");
        let written = serde_json::to_value(&spec).expect("writes");
        assert_eq!(written["bags"][0]["ordered"], true);
        assert!(written["bags"][2].get("ordered").is_none());
    }

    // ---- SHUFFLE

    #[test]
    fn a_shuffle_keeps_the_cards_and_follows_the_seed() {
        let shuffled = |seed: u64| {
            let mut deck_game = game(&deck_spec(json!([{ "name": "SHUFFLE", "do": [
                { "effect": "SHUFFLE", "at": deck() }
            ] }])));
            deck_game.set_seed(seed);
            take(&mut deck_game, "SHUFFLE");
            let order = pile(&deck_game, "DECK");
            let mut sorted: Vec<&str> = order.split(' ').collect();
            sorted.sort_unstable();
            assert_eq!(sorted, ["A", "A", "A", "B", "B", "C"], "{order}");
            order
        };
        assert_eq!(shuffled(21), shuffled(21));
        let orders: std::collections::BTreeSet<String> = (0..30).map(shuffled).collect();
        assert!(orders.len() > 10, "30 seeds gave {} orders", orders.len());
    }

    #[test]
    fn a_shuffle_is_recorded_without_its_order_and_uses_a_number_per_card_but_one() {
        let mut deck_game = game(&deck_spec(shuffle_and_draw(2)));
        deck_game.set_seed(4);
        take(&mut deck_game, "DRAW");
        assert_eq!(deck_game.state.draws, 5, "six cards: five numbers");
        let record = deck_game.history().last().expect("recorded");
        assert_eq!(record.shuffled, ["DECK"]);
        let written = serde_json::to_value(record).expect("serializes");
        assert_eq!(written["shuffled"], json!(["DECK"]), "names the pile and no order");
        assert_eq!(pile(&deck_game, "DECK").split(' ').count(), 4);
    }

    #[test]
    fn a_saved_match_draws_the_same_cards_as_one_never_stopped() {
        let actions = json!([
            { "name": "SHUFFLE", "do": [{ "effect": "SHUFFLE", "at": deck() }] },
            { "name": "DRAW", "do": [{ "effect": "MOVE", "from": deck(), "to": hand(), "count": 2 }] }
        ]);
        let mut straight = game(&deck_spec(actions.clone()));
        straight.set_seed(77);
        for name in ["SHUFFLE", "DRAW", "DRAW"] {
            take(&mut straight, name);
        }

        let mut first = game(&deck_spec(actions.clone()));
        first.set_seed(77);
        take(&mut first, "SHUFFLE");
        let saved = first.save_state().expect("saves");
        let mut resumed = game(&deck_spec(actions));
        resumed.restore_state(&saved).expect("loads");
        for name in ["DRAW", "DRAW"] {
            take(&mut resumed, name);
        }
        assert_eq!(pile(&straight, "DECK"), pile(&resumed, "DECK"));
        assert_eq!(
            straight.state.holders.get(&bag_position("HAND", Some("WHITE"))),
            resumed.state.holders.get(&bag_position("HAND", Some("WHITE")))
        );
        assert_eq!(straight.state.draws, resumed.state.draws);
    }

    #[test]
    fn a_pile_saves_in_order_and_loads_as_it_was() {
        let mut deck_game = game(&deck_spec(json!([{ "name": "SHUFFLE", "do": [
            { "effect": "SHUFFLE", "at": deck() }
        ] }])));
        deck_game.set_seed(8);
        take(&mut deck_game, "SHUFFLE");
        let before = pile(&deck_game, "DECK");
        assert_ne!(before, "A A A B B C", "this seed moves the cards");
        let saved = deck_game.save_state().expect("saves");
        deck_game.restore_state(&saved).expect("loads");
        assert_eq!(pile(&deck_game, "DECK"), before);
    }

    #[test]
    fn a_history_saved_before_chance_loads() {
        let old = json!({
            "player": "WHITE", "action": "MOVE", "chosen": [], "piece_code": "",
            "from": "", "to": "", "captures": false, "passed": true
        });
        let record: MoveRecord = serde_json::from_value(old).expect("an old record loads");
        assert!(record.rolled.is_empty() && record.shuffled.is_empty());
        let back = serde_json::to_value(&record).expect("writes");
        assert!(back.get("rolled").is_none() && back.get("shuffled").is_none());
    }
}
