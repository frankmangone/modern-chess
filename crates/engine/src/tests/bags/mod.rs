//! Bags: holders off the board, the hand among them. `bags` declares them, `removed_to` sends
//! the piece an effect removes to one, `captured_as` says which piece arrives.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Game, Piece};
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
    fn the_old_hand_forms_are_refused_naming_their_replacement() {
        let mut old_switch = spec(None, None);
        old_switch["hand_enabled"] = json!(true);
        let mut old_key = spec(None, None);
        old_key["hand"] = json!({ "enabled": true });
        let mut old_table = spec(None, None);
        old_table["pieces"][1]["demotes_to"] = json!("TAKER");
        for (form, spec, replacement) in [
            ("hand_enabled", old_switch, "\"bags\""),
            ("hand", old_key, "\"bags\""),
            ("demotes_to", old_table, "`captured_as`"),
        ] {
            let error = read(&spec)
                .err()
                .unwrap_or_else(|| panic!("{form} is refused"));
            assert!(
                matches!(error, GameSpecError::OldForm(_)),
                "{form}: {error}"
            );
            assert!(error.to_string().contains(replacement), "{form}: {error}");
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
}
