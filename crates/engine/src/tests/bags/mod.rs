//! Bags: holders off the board, the hand among them. `bags` declares them, `removed_to` sends
//! the piece an effect removes to one, `captured_as` says which piece arrives.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::places::holders;
    use crate::logic::{Game, Piece};
    use crate::shared::Position;
    use crate::specs::{parse_game_spec_str, GameSpec, GameSpecError};
    use crate::tests::support::{go, pick};

    fn read(spec: &Value) -> Result<GameSpec, GameSpecError> {
        parse_game_spec_str(&spec.to_string())
    }

    /// A 4×4 game: WHITE's TAKER steps [0, 1] and takes what stands there. `removed_to` is
    /// written on the MOVE effect when given.
    fn spec(bags: Option<Value>, removed_to: Option<Value>) -> Value {
        let mut effect =
            json!({ "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } });
        if let Some(value) = removed_to {
            effect["removed_to"] = value;
        }
        let mut spec = json!({
            "format_version": 1,
            "name": "TAKES",
            "board": { "dimensions": [4, 4] },
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]], "starting_positions": [] }
            ],
            "turns": { "order": ["WHITE", "BLACK"] },
            "pieces": [
                { "code": "TAKER", "name": "taker",
                  "moves": [{ "id": 0, "step": [0, 1], "do": [effect] }] },
                { "code": "PROMOTED", "name": "promoted", "captured_as": "TAKER", "moves": [] }
            ],
            "actions": [{ "name": "MOVE", "do": [
                { "choose": "PIECE", "where": { "owner": "ALLY" } },
                { "choose": "MOVE" }
            ] }]
        });
        if let Some(bags) = bags {
            spec["bags"] = bags;
        }
        spec
    }

    /// The bags `spec` writes by default: each player's hand.
    fn hand() -> Value {
        json!([{ "name": "HAND", "for": "EACH_PLAYER" }])
    }

    fn to_the_movers_hand() -> Value {
        json!({ "bag": "HAND", "of": "MOVER" })
    }

    #[test]
    fn bags_are_declared_per_player_or_shared() {
        let bags = json!([
            { "name": "HAND", "for": "EACH_PLAYER", "start": { "TAKER": 2 } },
            { "name": "SUPPLY", "capacity": 20, "start": { "TAKER": 19 } }
        ]);
        let parsed = read(&spec(Some(bags), None)).expect("reads");
        assert_eq!(parsed.bags.len(), 2);
        assert!(parsed.bags.first().is_some_and(|bag| bag.for_each_player()));
        assert!(parsed.bags.get(1).is_some_and(|bag| !bag.for_each_player()));
        assert!(parsed
            .bags
            .get(1)
            .is_some_and(|bag| bag.capacity == Some(20)));
        assert!(read(&spec(None, None)).expect("reads").bags.is_empty());
    }

    #[test]
    fn a_badly_written_bag_is_refused() {
        let cases = [
            ("a name twice", json!([{ "name": "A" }, { "name": "A" }])),
            ("an empty name", json!([{ "name": "" }])),
            ("an @ in a name", json!([{ "name": "A@B" }])),
            ("a capacity of 0", json!([{ "name": "A", "capacity": 0 }])),
            (
                "a start above the capacity",
                json!([{ "name": "A", "capacity": 1, "start": { "TAKER": 2 } }]),
            ),
            (
                "a start of an unknown piece",
                json!([{ "name": "A", "start": { "NOBODY": 1 } }]),
            ),
            (
                "a start of 0",
                json!([{ "name": "A", "start": { "TAKER": 0 } }]),
            ),
            (
                "an unknown `for`",
                json!([{ "name": "A", "for": "EVERYONE" }]),
            ),
            (
                "an unknown field",
                json!([{ "name": "A", "colour": "red" }]),
            ),
        ];
        for (what, bags) in cases {
            assert!(read(&spec(Some(bags), None)).is_err(), "{what} is refused");
        }
    }

    #[test]
    fn removed_to_names_a_bag_of_the_game() {
        assert!(read(&spec(Some(hand()), Some(to_the_movers_hand()))).is_ok());
        let to_the_enemy = json!({ "bag": "HAND", "of": "ENEMY" });
        assert!(read(&spec(Some(hand()), Some(to_the_enemy))).is_ok());
        let to_a_player = json!({ "bag": "HAND", "of": "BLACK" });
        assert!(read(&spec(Some(hand()), Some(to_a_player))).is_ok());
        let shared = json!([{ "name": "SUPPLY" }]);
        assert!(read(&spec(
            Some(shared.clone()),
            Some(json!({ "bag": "SUPPLY" }))
        ))
        .is_ok());

        let refused = [
            ("no bags at all", None, to_the_movers_hand()),
            (
                "an unknown bag",
                Some(hand()),
                json!({ "bag": "POOL", "of": "MOVER" }),
            ),
            (
                "a per-player bag with no `of`",
                Some(hand()),
                json!({ "bag": "HAND" }),
            ),
            (
                "a shared bag with an `of`",
                Some(shared),
                json!({ "bag": "SUPPLY", "of": "MOVER" }),
            ),
            (
                "an unknown player",
                Some(hand()),
                json!({ "bag": "HAND", "of": "GREEN" }),
            ),
            (
                "the old hand form",
                Some(hand()),
                json!({ "hand": "MOVER" }),
            ),
            ("a place", Some(hand()), json!({ "place": "TARGET" })),
        ];
        for (what, bags, removed_to) in refused {
            assert!(
                read(&spec(bags, Some(removed_to))).is_err(),
                "{what} is refused"
            );
        }
    }

    #[test]
    fn captured_as_must_name_a_piece() {
        let mut unknown = spec(Some(hand()), None);
        unknown["pieces"][1]["captured_as"] = json!("NOBODY");
        assert!(matches!(
            read(&unknown),
            Err(GameSpecError::UnknownPieceInRule(_))
        ));
    }

    /// A map of two points, `a` and `b`, joined, with `a` written as `place`.
    fn map_with(place: Value) -> Value {
        let mut spec = spec(None, None);
        spec["board"] = json!({ "shape": "map",
            "places": [place, { "name": "b", "at": [1, 0] }],
            "links": [["a", "b"]] });
        spec["players"] = json!([{ "name": "WHITE", "starting_positions": [] },
            { "name": "BLACK", "starting_positions": [] }]);
        spec["pieces"][0]["moves"] = json!([]);
        spec
    }

    #[test]
    fn a_map_place_may_hold_many() {
        let pit = json!({ "name": "a", "at": [0, 0], "capacity": 99, "start": { "TAKER": 4 } });
        let parsed = read(&map_with(pit)).expect("reads");
        let place = parsed
            .board
            .map
            .as_ref()
            .and_then(|map| map.places.first())
            .expect("a place");
        assert_eq!(place.capacity, Some(99));
        assert_eq!(place.start.get("TAKER"), Some(&4));

        let refused = [
            (
                "a capacity of 0",
                json!({ "name": "a", "at": [0, 0], "capacity": 0 }),
            ),
            (
                "a start on a place of one",
                json!({ "name": "a", "at": [0, 0], "start": { "TAKER": 1 } }),
            ),
            (
                "a start above the capacity",
                json!({ "name": "a", "at": [0, 0], "capacity": 2, "start": { "TAKER": 3 } }),
            ),
            (
                "a start of an unknown piece",
                json!({ "name": "a", "at": [0, 0], "capacity": 2, "start": { "NOBODY": 1 } }),
            ),
        ];
        for (what, place) in refused {
            assert!(read(&map_with(place)).is_err(), "{what} is refused");
        }
    }

    #[test]
    fn a_bag_and_a_place_cannot_share_a_name() {
        let mut shared = map_with(json!({ "name": "a", "at": [0, 0] }));
        shared["bags"] = json!([{ "name": "a" }]);
        assert!(read(&shared).is_err());
    }

    /// WHITE's TAKER at [1, 1] takes `victim` of `owner` at [1, 2] under `spec`; the code of
    /// every piece in WHITE's hand afterwards, with its count, sorted.
    fn hand_after_taking(spec: Value, victim: &str, owner: &str) -> Vec<(String, u32)> {
        let mut game = Game::from_spec(read(&spec).expect("reads"));
        game.state.pieces.clear();
        game.state
            .pieces
            .insert(pos![1, 1], Piece::new("TAKER", "WHITE"));
        game.state
            .pieces
            .insert(pos![1, 2], Piece::new(victim, owner));
        pick(&mut game, pos![1, 1]).expect("picks the taker");
        go(&mut game, pos![1, 2]).expect("takes");
        assert_eq!(
            game.state
                .pieces
                .get(&pos![1, 2])
                .map(|p| p.player.as_str()),
            Some("WHITE")
        );
        let mut held: Vec<(String, u32)> = game
            .hand()
            .get("WHITE")
            .into_iter()
            .flatten()
            .map(|(code, count)| (code.clone(), *count))
            .collect();
        held.sort();
        held
    }

    #[test]
    fn a_removed_enemy_piece_goes_to_the_hand_as_it_is_captured_as() {
        let sent = spec(Some(hand()), Some(to_the_movers_hand()));
        assert_eq!(
            hand_after_taking(sent, "PROMOTED", "BLACK"),
            [("TAKER".to_string(), 1)]
        );
        let plain = spec(Some(hand()), Some(to_the_movers_hand()));
        assert_eq!(
            hand_after_taking(plain, "TAKER", "BLACK"),
            [("TAKER".to_string(), 1)]
        );
    }

    #[test]
    fn a_removal_without_removed_to_puts_nothing_in_the_hand() {
        let unsent = spec(Some(hand()), None);
        assert!(hand_after_taking(unsent, "PROMOTED", "BLACK").is_empty());
    }

    #[test]
    fn the_movers_own_removed_piece_never_enters_its_hand() {
        let sent = spec(Some(hand()), Some(to_the_movers_hand()));
        assert!(hand_after_taking(sent, "PROMOTED", "WHITE").is_empty());
    }

    fn game_of(spec: &Value) -> Game {
        Game::from_spec(read(spec).expect("reads"))
    }

    #[test]
    fn a_game_starts_with_what_its_bags_and_holder_places_keep() {
        let mut written =
            map_with(json!({ "name": "a", "at": [0, 0], "capacity": 9, "start": { "TAKER": 4 } }));
        written["bags"] = json!([
            { "name": "HAND", "for": "EACH_PLAYER", "start": { "TAKER": 2 } },
            { "name": "SUPPLY", "start": { "TAKER": 19 } }
        ]);
        let game = game_of(&written);
        assert_eq!(game.state.hand_count("WHITE", "TAKER"), 2);
        assert_eq!(game.state.hand_count("BLACK", "TAKER"), 2);
        let mine = game.state.hand_of("WHITE").expect("a hand");
        assert_eq!(
            mine.entries().first().map(|held| held.owner.as_str()),
            Some("WHITE")
        );
        let supply = game
            .state
            .holders
            .get(&holders::bag_position("SUPPLY", None))
            .expect("a supply");
        assert_eq!(supply.total(), 19);
        assert_eq!(
            supply.entries().first().map(|held| held.owner.as_str()),
            Some("")
        );
        let pit = game.state.holders.get(&Position::new("a")).expect("a pit");
        assert_eq!(pit.count_of("TAKER"), 4);
        assert!(!game.state.holders.contains_key(&Position::new("b")));
    }

    #[test]
    fn holders_save_and_a_hand_saved_the_old_way_loads_into_the_bags() {
        let mut game = game_of(&spec(Some(hand()), None));
        game.state.put_in_hand("WHITE", "TAKER", 3);
        let saved = game.save_state().expect("saves");
        let json: Value = serde_json::from_str(&saved).expect("JSON");
        assert!(json.get("hand").is_none(), "the old key is not written");
        assert_eq!(json["holders"]["HAND@WHITE"][0]["count"], 3);
        let mut again = game_of(&spec(Some(hand()), None));
        again.restore_state(&saved).expect("loads");
        assert_eq!(again.state.hand_count("WHITE", "TAKER"), 3);

        let mut old: Value = serde_json::from_str(&saved).expect("JSON");
        old.as_object_mut().expect("an object").remove("holders");
        old["hand"] = json!({ "BLACK": { "TAKER": 1, "PROMOTED": 2 } });
        let mut loaded = game_of(&spec(Some(hand()), None));
        loaded.restore_state(&old.to_string()).expect("loads");
        assert_eq!(loaded.state.hand_count("BLACK", "TAKER"), 1);
        assert_eq!(loaded.state.hand_count("BLACK", "PROMOTED"), 2);
        assert_eq!(loaded.state.hand_count("WHITE", "TAKER"), 0);
        assert!(loaded.save_state().expect("saves").contains("HAND@BLACK"));
    }

    // ---- Effects on holders --------------------------------------------------------------

    fn bag(name: &str) -> Value {
        json!({ "bag": name, "of": "MOVER" })
    }

    fn shared(name: &str) -> Value {
        json!({ "bag": name })
    }

    /// The game of `spec` with the actions `actions` and the bags `bags`.
    fn holder_game(bags: Value, actions: Value) -> Game {
        let mut written = spec(Some(bags), None);
        written["actions"] = actions;
        game_of(&written)
    }

    /// A game whose one action `A` does `effects`, WHITE to move; the bags are each player's
    /// HAND and a SUPPLY of five TAKER (capacity 5).
    fn game_doing(effects: Value) -> Game {
        let bags = json!([
            { "name": "HAND", "for": "EACH_PLAYER" },
            { "name": "SUPPLY", "capacity": 5, "start": { "TAKER": 5 } }
        ]);
        holder_game(bags, json!([{ "name": "A", "do": effects }]))
    }

    /// Plays the action `A`; `false` when it is not offered.
    fn play_a(game: &mut Game) -> bool {
        use crate::logic::{Answer, GameTransition};
        game.transition(GameTransition::Choose(Answer::Action("A".to_string())))
            .is_ok()
    }

    fn supply(game: &Game) -> u32 {
        game.state
            .holders
            .get(&holders::bag_position("SUPPLY", None))
            .map_or(0, |held| held.total())
    }

    #[test]
    fn pieces_move_between_bags_and_take_the_owner_of_the_bag() {
        let mut game = game_doing(json!([{
            "effect": "MOVE", "from": shared("SUPPLY"), "to": bag("HAND"),
            "what": { "codes": ["TAKER"] }, "count": 2
        }]));
        assert!(play_a(&mut game));
        assert_eq!(supply(&game), 3);
        assert_eq!(game.state.hand_count("WHITE", "TAKER"), 2);
        let mine = game.state.hand_of("WHITE").expect("a hand");
        assert_eq!(
            mine.entries().first().map(|held| held.owner.as_str()),
            Some("WHITE")
        );
    }

    #[test]
    fn a_move_of_all_takes_everything_that_matches() {
        let mut game = game_doing(json!([{
            "effect": "MOVE", "from": shared("SUPPLY"),
            "to": { "bag": "HAND", "of": "ENEMY" }, "count": "ALL"
        }]));
        assert!(play_a(&mut game));
        assert_eq!(supply(&game), 0);
        assert_eq!(game.state.hand_count("BLACK", "TAKER"), 5);
        assert!(!game
            .state
            .holders
            .contains_key(&holders::bag_position("SUPPLY", None)));
    }

    #[test]
    fn a_move_that_takes_more_than_a_holder_holds_is_not_offered() {
        let mut game = game_doing(json!([{
            "effect": "MOVE", "from": shared("SUPPLY"), "to": bag("HAND"), "count": 6
        }]));
        assert!(!play_a(&mut game));
        assert_eq!(supply(&game), 5, "nothing was taken");
    }

    #[test]
    fn a_move_that_passes_a_capacity_is_not_offered() {
        let mut game = game_doing(json!([
            { "effect": "MOVE", "from": shared("SUPPLY"), "to": bag("HAND"), "count": 2 },
            { "effect": "MOVE", "from": bag("HAND"), "to": shared("SUPPLY"), "count": 3 }
        ]));
        assert!(!play_a(&mut game), "the hand holds 2, and 3 would not fit");
        let mut fits = game_doing(json!([
            { "effect": "MOVE", "from": shared("SUPPLY"), "to": bag("HAND"), "count": 2 },
            { "effect": "MOVE", "from": bag("HAND"), "to": shared("SUPPLY"), "count": 2 }
        ]));
        assert!(play_a(&mut fits));
        assert_eq!(supply(&fits), 5, "back where they were");
        let mut put = game_doing(json!([
            { "effect": "PLACE", "at": shared("SUPPLY"), "code": "TAKER" }
        ]));
        assert!(!play_a(&mut put), "the supply is full");
    }

    #[test]
    fn place_and_remove_work_on_holders() {
        let mut game = game_doing(json!([
            { "effect": "REMOVE", "at": shared("SUPPLY"), "count": 4, "removed_to": bag("HAND") },
            { "effect": "PLACE", "at": shared("SUPPLY"), "code": "PROMOTED", "count": 2 }
        ]));
        assert!(play_a(&mut game));
        assert_eq!(supply(&game), 3);
        assert_eq!(game.state.hand_count("WHITE", "TAKER"), 4);
        let supply_now = game
            .state
            .holders
            .get(&holders::bag_position("SUPPLY", None));
        assert_eq!(supply_now.map(|held| held.count_of("PROMOTED")), Some(2));
    }

    #[test]
    fn a_piece_moved_into_a_bag_becomes_a_count_and_out_of_one_a_new_piece() {
        use crate::tests::support::{go, pick};
        let bags = json!([
            { "name": "HAND", "for": "EACH_PLAYER" },
            { "name": "SUPPLY", "start": { "TAKER": 2 } }
        ]);
        let actions = json!([
            { "name": "STORE", "do": [
                { "choose": "PIECE", "where": { "owner": "ALLY" } },
                { "effect": "MOVE", "from": { "place": "START" }, "to": bag("HAND") }
            ] },
            { "name": "TAKE", "do": [
                { "choose": "PLACE", "in": { "region": "BOARD", "where": { "empty": true } } },
                { "effect": "MOVE", "from": shared("SUPPLY"), "to": { "place": "TARGET" } }
            ] }
        ]);
        let mut game = holder_game(bags.clone(), actions.clone());
        game.state.pieces.clear();
        let mut worn = Piece::new("TAKER", "WHITE");
        worn.total_moves = 3;
        game.state.pieces.insert(pos![1, 1], worn);
        game.state
            .pieces
            .insert(pos![3, 3], Piece::new("PROMOTED", "BLACK"));
        pick(&mut game, pos![1, 1]).expect("picks");
        assert!(
            game.state.pieces.get(&pos![1, 1]).is_none(),
            "it left the board"
        );
        assert_eq!(
            game.state.hand_count("WHITE", "TAKER"),
            1,
            "and is a count in the hand"
        );

        let mut game = holder_game(bags, actions);
        game.state.pieces.clear();
        game.state
            .pieces
            .insert(pos![3, 3], Piece::new("PROMOTED", "BLACK"));
        // WHITE has nothing to store; the TAKE action puts a supply piece on an empty square.
        game.work_out_ways();
        go(&mut game, pos![0, 0]).expect("takes");
        let dropped = game.state.pieces.get(&pos![0, 0]).expect("a piece");
        assert_eq!(
            dropped.player, "WHITE",
            "nobody's pieces become the mover's"
        );
        assert_eq!(dropped.total_moves, 0);
        assert_eq!(supply(&game), 1);
    }

    #[test]
    fn a_removed_piece_goes_to_a_shared_bag_as_nobodys() {
        let bags = json!([{ "name": "BOX" }]);
        let mut written = spec(Some(bags), Some(json!({ "bag": "BOX" })));
        written["pieces"][1]["captured_as"] = json!("TAKER");
        let mut game = game_of(&written);
        game.state.pieces.clear();
        game.state
            .pieces
            .insert(pos![1, 1], Piece::new("TAKER", "WHITE"));
        game.state
            .pieces
            .insert(pos![1, 2], Piece::new("PROMOTED", "BLACK"));
        pick(&mut game, pos![1, 1]).expect("picks");
        go(&mut game, pos![1, 2]).expect("takes");
        let boxed = game
            .state
            .holders
            .get(&holders::bag_position("BOX", None))
            .expect("a box");
        assert_eq!(boxed.entries().len(), 1);
        assert_eq!(
            boxed
                .entries()
                .first()
                .map(|held| (held.owner.as_str(), held.code.as_str())),
            Some(("", "TAKER"))
        );
    }

    #[test]
    fn a_way_records_what_it_does_to_holders() {
        let mut game = game_doing(json!([{
            "effect": "MOVE", "from": shared("SUPPLY"), "to": bag("HAND"), "count": 2
        }]));
        game.work_out_ways();
        let ways = game.state.ways.clone().expect("ways");
        let way = ways.iter().find(|way| way.action == "A").expect("offered");
        assert_eq!(way.holder_changes.len(), 2);
        assert_eq!(
            way.holder_changes
                .iter()
                .map(|change| change.delta)
                .sum::<i32>(),
            0
        );
    }

    // ---- Counts, FOLLOW and one-way links --------------------------------------------------

    /// A ring of three places, `a` -> `b` -> `c` -> `a`, each holding up to nine, `a` starting
    /// with two TAKER, with the steps `steps` of the one action `A`.
    fn ring_with(steps: Value) -> Value {
        let mut spec = spec(None, None);
        spec["board"] = json!({ "shape": "map",
            "places": [
                { "name": "a", "at": [0, 0], "capacity": 9, "start": { "TAKER": 2 } },
                { "name": "b", "at": [1, 0], "capacity": 9 },
                { "name": "c", "at": [2, 0], "capacity": 9 }
            ],
            "links": [
                { "from": "a", "to": "b" }, { "from": "b", "to": "c" }, { "from": "c", "to": "a" }
            ] });
        spec["players"] = json!([{ "name": "WHITE", "starting_positions": [] },
            { "name": "BLACK", "starting_positions": [] }]);
        spec["pieces"][0]["moves"] = json!([]);
        spec["zones"] = json!([{ "name": "SKIP", "places": { "WHITE": ["c"], "BLACK": ["a"] } }]);
        spec["actions"] = json!([{ "name": "A", "do": steps }]);
        spec
    }

    /// The steps of an action that chooses a place and puts a TAKER on each place of the FOLLOW
    /// from it with `steps` and `skipping`.
    fn put_along(steps: Value, skipping: Option<&str>) -> Value {
        let mut region =
            json!({ "region": "FOLLOW", "from": { "place": "TARGET" }, "steps": steps });
        if let Some(zone) = skipping {
            region["skipping"] = json!(zone);
        }
        json!([
            { "choose": "PLACE", "in": { "region": "BOARD" } },
            { "effect": "FOR_EACH", "in": region,
              "do": [{ "effect": "PLACE", "at": { "place": "EACH" }, "code": "TAKER" }] }
        ])
    }

    /// What `a`, `b` and `c` keep after the action of `spec` is played from the place `from`.
    fn ring_after(spec: &Value, from: &str) -> [u32; 3] {
        use crate::logic::{Answer, GameTransition};
        let mut game = game_of(spec);
        game.transition(GameTransition::Choose(Answer::Place(Position::new(from))))
            .expect("the action is played");
        let kept = |name: &str| {
            game.state
                .holders
                .get(&Position::new(name))
                .map_or(0, |held| held.total())
        };
        [kept("a"), kept("b"), kept("c")]
    }

    #[test]
    fn a_one_way_link_goes_one_way_and_is_written_back() {
        let parsed = read(&ring_with(json!([]))).expect("reads");
        let map = parsed.board.map.as_ref().expect("a map");
        assert!(map.lines.is_empty());
        assert_eq!(map.one_way.len(), 3);
        assert_eq!(map.links["next"], vec![vec![1], vec![2], vec![0]]);
        let written = serde_json::to_string(&parsed).expect("writes");
        let again = parse_game_spec_str(&written).expect("reads back");
        assert_eq!(
            again.board.map.as_ref().map(|map| map.one_way.clone()),
            Some(map.one_way.clone())
        );
        for bad in [
            json!({ "from": "a", "to": "a" }),
            json!({ "from": "a", "to": "nowhere" }),
            json!({ "from": "a", "to": "b", "lable": "next" }),
        ] {
            let mut spec = ring_with(json!([]));
            spec["board"]["links"] = json!([bad]);
            assert!(read(&spec).is_err());
        }
    }

    #[test]
    fn follow_visits_the_next_places_going_round() {
        let steps = |count: u32| ring_with(put_along(json!(count), None));
        assert_eq!(ring_after(&steps(2), "a"), [2, 1, 1]);
        assert_eq!(
            ring_after(&steps(4), "a"),
            [3, 2, 1],
            "round again: b and c, a, b"
        );
        assert_eq!(ring_after(&steps(1), "c"), [3, 0, 0]);
    }

    #[test]
    fn follow_skips_the_places_of_a_zone_for_the_mover_without_counting_them() {
        let spec = ring_with(put_along(json!(2), Some("SKIP")));
        assert_eq!(
            ring_after(&spec, "a"),
            [3, 1, 0],
            "WHITE passes over c: b, a"
        );
    }

    #[test]
    fn a_count_can_be_read_from_the_position() {
        // The pieces at `a`: two. From `b`, two steps: c and a.
        let counted = json!({ "count": { "in": { "region": "AT", "at": { "place": "TARGET" } } } });
        let from_the_place_itself = ring_with(put_along(counted, None));
        assert_eq!(ring_after(&from_the_place_itself, "a"), [2, 1, 1]);
        assert_eq!(
            ring_after(&from_the_place_itself, "b"),
            [2, 0, 0],
            "none at b: no step"
        );
    }

    #[test]
    fn follow_and_counts_are_refused_where_they_mean_nothing() {
        assert!(
            read(&ring_with(put_along(json!("ALL"), None))).is_err(),
            "steps of ALL"
        );
        assert!(
            read(&ring_with(put_along(json!(0), None))).is_err(),
            "steps of 0"
        );
        assert!(
            read(&ring_with(put_along(json!(1), Some("NOWHERE")))).is_err(),
            "an unknown zone"
        );
        let mut two_ways = ring_with(put_along(json!(1), None));
        two_ways["board"]["links"] = json!([["a", "b"], ["b", "c"]]);
        assert!(
            read(&two_ways).is_err(),
            "`next` leads to several places from b"
        );
        let mut grid = spec(Some(hand()), None);
        grid["actions"] = json!([{ "name": "A", "do": put_along(json!(1), None) }]);
        assert!(read(&grid).is_err(), "a grid has no links to follow");
    }

    #[test]
    fn a_count_in_an_effect_is_a_number_all_or_counted() {
        let mut spec = ring_with(json!([]));
        spec["actions"][0]["do"] = json!([
            { "choose": "PLACE", "in": { "region": "BOARD" } },
            { "effect": "MOVE", "from": { "place": "TARGET" }, "to": { "place": "TARGET" },
              "count": { "count": { "in": { "region": "AT", "at": { "place": "TARGET" } } } } }
        ]);
        assert!(read(&spec).is_ok());
        spec["actions"][0]["do"][1]["count"] =
            json!({ "count": { "in": { "region": "NOWHERE" } } });
        assert!(read(&spec).is_err());
        spec["actions"][0]["do"][1]["count"] = json!({ "total": 2 });
        assert!(read(&spec).is_err());
    }

    #[test]
    fn the_until_of_a_repeat_reads_what_a_bag_keeps() {
        // A slider that stops once the SUPPLY bag keeps a stone: with one in it the slider
        // reaches one square, with none it runs to the edge.
        let mut slider = spec(
            Some(json!([{ "name": "SUPPLY", "start": { "TAKER": 1 } }])),
            None,
        );
        slider["pieces"][0]["moves"][0]["repeat"] = json!({
            "loop": true,
            "until": { "test": "COUNT", "in": { "region": "AT", "at": { "bag": "SUPPLY" } } }
        });
        let reaches = |spec: &Value| {
            let mut game = Game::from_spec(read(spec).expect("reads"));
            game.state.pieces.clear();
            game.state
                .pieces
                .insert(Position::new("0,0"), Piece::new("TAKER", "WHITE"));
            game.work_out_ways();
            crate::tests::support::moves_from(&mut game, Position::new("0,0"))
        };

        assert_eq!(reaches(&slider), vec![Position::new("0,1")]);

        slider["bags"][0]["start"] = json!({});
        assert_eq!(reaches(&slider).len(), 3);
    }

    /// A game whose one action puts the piece you pick into your own hand (`own`), or the
    /// enemy's piece into yours (`!own`, by way of a removal): the board loses a piece either way.
    fn tidying_game(own: bool) -> Game {
        let mut written = spec(Some(hand()), None);
        written["actions"] = json!([{ "name": "STORE", "do": [
            { "choose": "PIECE", "where": { "owner": if own { "ALLY" } else { "ENEMY" } } },
            { "effect": "MOVE", "from": { "place": "START" }, "to": { "bag": "HAND", "of": "MOVER" } }
        ] }]);
        let mut game = Game::from_spec(read(&written).expect("reads"));
        game.state.pieces.clear();
        game.state
            .pieces
            .insert(pos![1, 1], Piece::new("TAKER", "WHITE"));
        game.state
            .pieces
            .insert(pos![2, 2], Piece::new("TAKER", "BLACK"));
        game
    }

    #[test]
    fn a_piece_put_into_its_owners_own_bag_is_not_counted_as_captured() {
        let mut game = tidying_game(true);
        pick(&mut game, pos![1, 1]).expect("stores the piece");
        assert!(game.state.pieces.get(&pos![1, 1]).is_none());
        assert_eq!(
            game.hand().get("WHITE").and_then(|h| h.get("TAKER")),
            Some(&1)
        );
        assert!(
            game.state.captured.is_empty(),
            "kept in its owner's hand, it was not lost: {:?}",
            game.state.captured
        );
    }

    #[test]
    fn an_enemy_piece_taken_into_the_movers_bag_is_counted_as_captured() {
        let mut game = tidying_game(false);
        pick(&mut game, pos![2, 2]).expect("takes the piece");
        assert_eq!(
            game.state
                .captured
                .get("BLACK")
                .and_then(|c| c.get("TAKER")),
            Some(&1)
        );
    }
}
