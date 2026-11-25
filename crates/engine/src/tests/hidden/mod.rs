//! Hidden information, at the engine: what a bag may declare, and the rules it then reads the
//! spec by. Who sees what of a running game is the session's, and tested there.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::specs::{parse_game_spec_str, GameSpecError};

    fn read(spec: &Value) -> Result<crate::specs::GameSpec, GameSpecError> {
        parse_game_spec_str(&spec.to_string())
    }

    fn accepted(spec: &Value) {
        if let Err(error) = read(spec) {
            panic!("the spec is refused: {error}");
        }
    }

    fn refusal(spec: &Value) -> String {
        read(spec).expect_err("the spec is refused").to_string()
    }

    /// A card game: a deck, each player's hand, a shared row, with `bags` replacing the two
    /// that hide and `actions` given.
    fn card_spec(hand: Value, deck: Value, actions: Value) -> Value {
        let mut hand_bag = json!({ "name": "HAND", "for": "EACH_PLAYER" });
        let mut deck_bag = json!({ "name": "DECK", "ordered": true, "start": { "A": 3, "B": 2 } });
        for (bag, extra) in [(&mut hand_bag, hand), (&mut deck_bag, deck)] {
            if let (Some(bag), Some(extra)) = (bag.as_object_mut(), extra.as_object()) {
                bag.extend(extra.clone());
            }
        }
        json!({
            "format_version": 1,
            "name": "CARDS",
            "board": { "dimensions": [2, 2] },
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "pieces": [
                { "code": "A", "name": "a", "moves": [] },
                { "code": "B", "name": "b", "moves": [] }
            ],
            "bags": [hand_bag, deck_bag, { "name": "ROW" }],
            "actions": actions
        })
    }

    fn deck() -> Value {
        json!({ "bag": "DECK" })
    }

    fn mine() -> Value {
        json!({ "bag": "HAND", "of": "MOVER" })
    }

    fn theirs() -> Value {
        json!({ "bag": "HAND", "of": "ENEMY" })
    }

    fn draw() -> Value {
        json!([{ "name": "DRAW", "do": [
            { "effect": "MOVE", "from": deck(), "to": mine(), "count": 2 }
        ] }])
    }

    fn own_hand() -> Value {
        json!({ "visible": "OWNER" })
    }

    fn secret_deck() -> Value {
        json!({ "visible": "NONE" })
    }

    fn strict_deck() -> Value {
        json!({ "visible": "NONE", "strict": true })
    }

    // ---- the declaration

    #[test]
    fn a_bag_can_say_who_sees_it_and_whether_its_count_shows() {
        let spec = card_spec(
            json!({ "visible": "OWNER", "shows_count": false }),
            json!({ "visible": "NONE", "shows_count": true, "strict": true }),
            draw(),
        );
        let game = read(&spec).expect("the spec reads");
        let hand = game
            .bags
            .iter()
            .find(|bag| bag.name == "HAND")
            .expect("hand");
        assert_eq!(
            serde_json::to_value(hand).expect("json")["visible"],
            "OWNER"
        );
        assert_eq!(
            serde_json::to_value(hand).expect("json")["shows_count"],
            false
        );
        let deck = game
            .bags
            .iter()
            .find(|bag| bag.name == "DECK")
            .expect("deck");
        assert_eq!(serde_json::to_value(deck).expect("json")["strict"], true);
    }

    #[test]
    fn a_bag_that_hides_nothing_writes_nothing_new() {
        let spec = card_spec(json!({}), json!({}), draw());
        let game = read(&spec).expect("the spec reads");
        for bag in &game.bags {
            let json = serde_json::to_value(bag).expect("json");
            for key in ["visible", "shows_count", "strict"] {
                assert!(json.get(key).is_none(), "{key} is left out of {json}");
            }
        }
    }

    #[test]
    fn visible_all_is_the_default_and_may_be_written() {
        accepted(&card_spec(json!({ "visible": "ALL" }), json!({}), draw()));
    }

    #[test]
    fn an_unknown_visibility_is_refused() {
        let spec = card_spec(json!({ "visible": "FRIENDS" }), json!({}), draw());
        assert!(!refusal(&spec).is_empty());
    }

    #[test]
    fn shows_count_on_a_bag_everyone_sees_is_refused() {
        for written in [
            json!({ "shows_count": true }),
            json!({ "shows_count": false }),
        ] {
            let text = refusal(&card_spec(written, json!({}), draw()));
            assert!(
                text.contains("HAND") && text.contains("shows_count"),
                "{text}"
            );
        }
    }

    #[test]
    fn strict_on_a_bag_everyone_sees_is_refused() {
        let text = refusal(&card_spec(json!({}), json!({ "strict": true }), draw()));
        assert!(text.contains("DECK") && text.contains("strict"), "{text}");
    }

    #[test]
    fn an_owner_only_bag_that_is_shared_is_refused() {
        // A shared bag belongs to nobody, so there is no owner to show it to.
        let text = refusal(&card_spec(json!({}), json!({ "visible": "OWNER" }), draw()));
        assert!(text.contains("DECK") && text.contains("OWNER"), "{text}");
    }

    #[test]
    fn a_hidden_bag_that_is_shared_may_be_hidden_from_all() {
        accepted(&card_spec(json!({}), secret_deck(), draw()));
    }

    #[test]
    fn a_place_that_holds_many_takes_no_visibility() {
        let mut spec = card_spec(json!({}), json!({}), draw());
        spec["board"] = json!({ "map": {
            "places": [{ "name": "CLEARING", "kind": "POINT", "at": [0, 0], "capacity": 4,
                          "visible": "NONE" }],
            "links": []
        } });
        assert!(
            read(&spec).is_err(),
            "a clearing is public, and takes no `visible`"
        );
    }

    // ---- rule 4: a HELD choice only names a bag the mover can see

    fn held_from(bag: Value) -> Value {
        json!([{ "name": "PLAY", "do": [
            { "choose": "HELD", "in": bag },
            { "effect": "MOVE", "from": { "place": "START" }, "to": { "bag": "ROW" } }
        ] }])
    }

    #[test]
    fn a_held_choice_in_the_movers_own_hidden_hand_is_allowed() {
        accepted(&card_spec(own_hand(), json!({}), held_from(mine())));
    }

    #[test]
    fn a_held_choice_in_the_opponents_hidden_hand_is_refused() {
        let text = refusal(&card_spec(own_hand(), json!({}), held_from(theirs())));
        assert!(text.contains("HELD") && text.contains("HAND"), "{text}");
    }

    #[test]
    fn a_held_choice_in_a_hand_everyone_sees_is_allowed() {
        accepted(&card_spec(json!({}), json!({}), held_from(theirs())));
    }

    #[test]
    fn a_held_choice_in_a_bag_nobody_sees_is_refused() {
        let mut spec = card_spec(json!({}), json!({}), held_from(mine()));
        spec["bags"] = json!([
            { "name": "HAND", "for": "EACH_PLAYER", "visible": "NONE" },
            { "name": "ROW" }
        ]);
        assert!(refusal(&spec).contains("HELD"));
    }

    #[test]
    fn a_held_choice_in_a_named_players_hand_is_allowed_to_that_player_alone() {
        let white_hand = json!({ "bag": "HAND", "of": "WHITE" });
        let play = |players: Value| {
            json!([{ "name": "PLAY", "players": players, "do": [
                { "choose": "HELD", "in": white_hand },
                { "effect": "MOVE", "from": { "place": "START" }, "to": { "bag": "ROW" } }
            ] }])
        };
        accepted(&card_spec(own_hand(), json!({}), play(json!(["WHITE"]))));
        assert!(read(&card_spec(own_hand(), json!({}), play(json!([])))).is_err());
        assert!(read(&card_spec(own_hand(), json!({}), play(json!(["BLACK"])))).is_err());
    }

    #[test]
    fn the_held_rule_holds_without_strict() {
        // Not strict, and still refused: a choice shows its options outright.
        let mut spec = card_spec(own_hand(), json!({}), held_from(theirs()));
        spec["bags"][0] = json!({ "name": "HAND", "for": "EACH_PLAYER", "visible": "OWNER" });
        assert!(read(&spec).is_err());
    }

    // ---- rule 5: strict bags are not read by the rules

    /// Whether an action with `steps` is read, for a strict deck and for the same deck not strict.
    fn strict_and_lax(steps: Value) -> (bool, bool) {
        let actions = json!([{ "name": "GO", "do": steps }]);
        (
            read(&card_spec(json!({}), strict_deck(), actions.clone())).is_ok(),
            read(&card_spec(json!({}), secret_deck(), actions)).is_ok(),
        )
    }

    fn deck_holds_an_a() -> Value {
        json!({ "test": "COUNT", "in": { "region": "AT", "at": deck() },
                "where": { "codes": ["A"] } })
    }

    #[test]
    fn a_test_of_what_a_strict_deck_holds_is_refused_and_a_lax_one_is_not() {
        assert_eq!(strict_and_lax(json!([deck_holds_an_a()])), (false, true));
    }

    #[test]
    fn an_if_over_a_strict_deck_is_refused() {
        let steps = json!([{ "if": deck_holds_an_a(),
            "then": [{ "effect": "MOVE", "from": deck(), "to": mine() }] }]);
        assert_eq!(strict_and_lax(steps), (false, true));
    }

    #[test]
    fn the_refusal_names_the_bag_and_says_what_to_do() {
        let actions = json!([{ "name": "GO", "do": [deck_holds_an_a()] }]);
        let text = refusal(&card_spec(json!({}), strict_deck(), actions));
        assert!(text.contains("DECK") && text.contains("strict"), "{text}");
        assert!(text.contains("action GO"), "{text}");
    }

    #[test]
    fn a_count_of_a_strict_deck_is_refused_without_shows_count() {
        let total = json!({ "test": "COUNT", "in": { "region": "AT", "at": deck() }, "min": 2 });
        let actions = json!([{ "name": "GO", "do": [total] }]);
        let hides_count = json!({ "visible": "NONE", "strict": true, "shows_count": false });
        assert!(read(&card_spec(json!({}), hides_count, actions.clone())).is_err());
        accepted(&card_spec(json!({}), secret_deck(), actions));
    }

    #[test]
    fn the_total_of_a_strict_deck_may_be_read_when_its_count_shows() {
        let total = json!({ "test": "COUNT", "in": { "region": "AT", "at": deck() }, "min": 2 });
        let actions = json!([{ "name": "GO", "do": [total] }]);
        let shows = json!({ "visible": "NONE", "strict": true, "shows_count": true });
        accepted(&card_spec(json!({}), shows.clone(), actions));
        // The total, not what makes it up.
        let by_code = json!([{ "name": "GO", "do": [deck_holds_an_a()] }]);
        assert!(read(&card_spec(json!({}), shows, by_code)).is_err());
    }

    #[test]
    fn a_strict_deck_counted_by_an_effects_count_follows_the_same_rule() {
        let draw_as_many = |filter: Value| {
            json!([{ "effect": "MOVE", "from": deck(), "to": mine(),
                     "count": { "count": { "in": { "region": "AT", "at": deck() },
                                           "where": filter } } }])
        };
        assert_eq!(
            strict_and_lax(draw_as_many(json!({ "codes": ["A"] }))),
            (false, true)
        );
    }

    #[test]
    fn drawing_from_a_strict_deck_is_writable() {
        for steps in [
            json!([{ "effect": "SHUFFLE", "at": deck() },
                   { "effect": "MOVE", "from": deck(), "to": mine(), "count": 2 }]),
            json!([{ "effect": "MOVE", "from": deck(), "to": mine(), "count": "ALL" }]),
            json!([{ "effect": "REMOVE", "at": deck(), "count": 1 }]),
            json!([{ "effect": "PLACE", "at": deck(), "code": "A" }]),
            json!([{ "effect": "MOVE", "from": mine(), "to": deck(), "count": "ALL" }]),
            json!([{ "effect": "MOVE", "from": mine(), "to": { "bag": "ROW" },
                    "removed_to": deck() }]),
        ] {
            let (strict, _) = strict_and_lax(steps.clone());
            assert!(strict, "{steps} is writable on a strict deck");
        }
    }

    #[test]
    fn taking_a_chosen_kind_from_a_strict_deck_is_refused() {
        let by_code = json!([{ "effect": "MOVE", "from": deck(), "to": mine(),
                               "what": { "codes": ["A"] } }]);
        // A pile takes no `what` at all, so this is also refused when it is not strict. The
        // pair that tells strict from lax is a bag that is not a pile.
        assert!(read(&card_spec(
            json!({}),
            strict_deck(),
            json!([{ "name": "GO", "do": by_code }])
        ))
        .is_err());
    }

    #[test]
    fn taking_a_kind_from_a_strict_hand_is_refused_and_from_a_lax_one_is_not() {
        let take_a = json!([{ "name": "GO", "do": [
            { "effect": "MOVE", "from": theirs(), "to": mine(), "what": { "codes": ["A"] } }
        ] }]);
        let strict = json!({ "visible": "OWNER", "strict": true });
        assert!(read(&card_spec(strict, json!({}), take_a.clone())).is_err());
        accepted(&card_spec(own_hand(), json!({}), take_a));
    }

    #[test]
    fn a_strict_hand_may_be_read_by_its_owner() {
        let strict = json!({ "visible": "OWNER", "strict": true });
        let own_has_an_a = json!({ "test": "COUNT", "in": { "region": "AT", "at": mine() },
                                    "where": { "codes": ["A"] } });
        accepted(&card_spec(
            strict,
            json!({}),
            json!([{ "name": "GO", "do": [own_has_an_a] }]),
        ));
    }

    #[test]
    fn a_strict_hand_may_not_be_read_by_the_other_player() {
        let strict = json!({ "visible": "OWNER", "strict": true });
        let theirs_has_an_a = json!({ "test": "COUNT", "in": { "region": "AT", "at": theirs() },
                                       "where": { "codes": ["A"] } });
        let actions = json!([{ "name": "GO", "do": [theirs_has_an_a] }]);
        assert!(read(&card_spec(strict, json!({}), actions)).is_err());
    }

    #[test]
    fn a_strict_hand_of_a_named_player_is_read_only_by_an_action_for_that_player() {
        let strict = json!({ "visible": "OWNER", "strict": true });
        let white_has_an_a = json!({ "test": "COUNT",
            "in": { "region": "AT", "at": { "bag": "HAND", "of": "WHITE" } },
            "where": { "codes": ["A"] } });
        let action = |players: Value| json!([{ "name": "GO", "players": players, "do": [white_has_an_a.clone()] }]);
        accepted(&card_spec(
            strict.clone(),
            json!({}),
            action(json!(["WHITE"])),
        ));
        assert!(read(&card_spec(strict.clone(), json!({}), action(json!([])))).is_err());
        assert!(read(&card_spec(strict, json!({}), action(json!(["BLACK"])))).is_err());
    }

    #[test]
    fn every_action_must_and_stages_follow_the_same_rule() {
        let mut spec = card_spec(
            json!({}),
            strict_deck(),
            json!([{ "name": "GO", "do": [] }]),
        );
        spec["every_action"] = json!([{ "do": [deck_holds_an_a()] }]);
        assert!(refusal(&spec).contains("every_action"));

        let mut spec = card_spec(
            json!({}),
            strict_deck(),
            json!([{ "name": "GO", "do": [] }]),
        );
        spec["must"] = json!([deck_holds_an_a()]);
        assert!(refusal(&spec).contains("must"));

        let mut spec = card_spec(
            json!({}),
            strict_deck(),
            json!([{ "name": "GO", "do": [] }]),
        );
        spec["turns"]["stages"] = json!([{ "name": "AFTER", "only": deck_holds_an_a() }]);
        assert!(refusal(&spec).contains("stage"));
    }

    #[test]
    fn an_end_check_may_read_a_strict_deck() {
        // It ends the game, which shows everything.
        let mut spec = card_spec(json!({}), strict_deck(), draw());
        spec["end_checks"] = json!([{ "when": deck_holds_an_a().clone(),
                                       "then": { "outcome": "DRAW" } }]);
        accepted(&spec);
    }

    #[test]
    fn a_strict_deck_is_not_read_through_a_for_each_or_a_test_on_a_place() {
        let steps =
            json!([{ "effect": "FOR_EACH", "in": { "region": "AT", "at": deck() }, "do": [] }]);
        assert_eq!(strict_and_lax(steps), (false, true));
        let same = json!([{ "test": "SAME", "a": deck(), "b": mine() }]);
        assert_eq!(strict_and_lax(same), (false, true));
    }

    #[test]
    fn a_strict_bag_everyone_sees_does_not_exist_so_nothing_there_is_refused() {
        // Reading a public bag is as it always was.
        let looks = json!([{ "name": "GO", "do": [
            { "test": "COUNT", "in": { "region": "AT", "at": { "bag": "ROW" } },
              "where": { "codes": ["A"] } }
        ] }]);
        accepted(&card_spec(json!({}), strict_deck(), looks));
    }

    #[test]
    fn repeats_compares_the_whole_position_so_a_strict_bag_forbids_it() {
        let repeats = json!([{ "name": "GO", "do": [{ "test": "REPEATS" }] }]);
        assert!(read(&card_spec(json!({}), strict_deck(), repeats.clone())).is_err());
        // Not strict, or not hidden: as it always was.
        accepted(&card_spec(json!({}), secret_deck(), repeats.clone()));
        accepted(&card_spec(json!({}), json!({}), repeats));
        // An end check may ask: it ends the game.
        let mut spec = card_spec(json!({}), strict_deck(), draw());
        spec["end_checks"] = json!([{ "when": { "test": "REPEATS" },
                                       "then": { "outcome": "DRAW" } }]);
        accepted(&spec);
    }

    #[test]
    fn a_place_choice_over_a_strict_bag_is_refused_even_when_its_count_shows() {
        let steps = json!([{ "choose": "PLACE", "in": { "region": "AT", "at": deck() } }]);
        assert_eq!(strict_and_lax(steps), (false, true));
    }
}
