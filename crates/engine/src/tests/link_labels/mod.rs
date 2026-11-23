//! Link labels of a spec's own: a written link may carry a label, and steps, `AROUND` and
//! `FOLLOW` may follow it.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::specs::{parse_game_spec_str, GameSpec, GameSpecError, MapSpec};
    use crate::tests::support::move_action;

    fn point(name: &str, at: [f64; 2]) -> Value {
        json!({ "name": name, "at": at })
    }

    /// Three points `a`, `b`, `c` in a row, joined by `links`, and a PROBE whose move steps
    /// through `label` and carries the tests `extra`.
    fn game(links: Value, label: &str, extra: Vec<Value>) -> Value {
        let mut steps = extra;
        steps.push(json!({ "effect": "MOVE", "from": { "place": "START" },
                           "to": { "place": "TARGET" } }));
        json!({
            "format_version": 1,
            "name": "LINK LABELS",
            "board": {
                "shape": "map",
                "places": [point("a", [0.0, 0.0]), point("b", [1.0, 0.0]), point("c", [2.0, 0.0])],
                "links": links
            },
            "turns": { "order": ["A", "B"] },
            "players": [
                { "name": "A", "starting_positions": [] },
                { "name": "B", "starting_positions": [] }
            ],
            "pieces": [{ "code": "PROBE", "name": "probe",
                         "moves": [{ "id": 0, "step": label, "do": steps }] }],
            "actions": [move_action()]
        })
    }

    fn plain(links: Value) -> Value {
        game(links, "next", vec![])
    }

    fn read(spec: &Value) -> Result<GameSpec, GameSpecError> {
        parse_game_spec_str(&spec.to_string())
    }

    fn map(spec: &GameSpec) -> &MapSpec {
        spec.board.map.as_deref().expect("a map")
    }

    fn refused(spec: &Value) -> String {
        match read(spec) {
            Err(error) => error.to_string(),
            Ok(_) => panic!("the spec should be refused"),
        }
    }

    #[test]
    fn a_written_label_is_a_link_of_the_map() {
        let spec = read(&plain(json!([
            { "from": "a", "to": "b", "label": "across" },
            { "between": ["a", "c"], "label": "across" },
            { "from": "b", "to": "c" }
        ])))
        .expect("the labelled links read");
        let map = map(&spec);
        let (a, b, c) = (
            map.index("a").unwrap(),
            map.index("b").unwrap(),
            map.index("c").unwrap(),
        );
        assert_eq!(map.links["across"][a], vec![b, c]);
        assert_eq!(map.links["across"][b], Vec::<usize>::new());
        assert_eq!(map.links["across"][c], vec![a], "between goes both ways");
        assert_eq!(map.links["next"][b], vec![c], "an unlabelled link is next");
        assert_eq!(map.links["next"][a], Vec::<usize>::new());
    }

    #[test]
    fn a_labelled_link_is_no_line() {
        let spec = read(&plain(json!([
            { "from": "a", "to": "b", "label": "across" },
            { "between": ["a", "c"], "label": "across" }
        ])))
        .expect("reads");
        let map = map(&spec);
        assert!(map.lines.is_empty() && map.one_way.is_empty());
        assert_eq!(map.labelled.len(), 2);
    }

    #[test]
    fn a_step_an_around_and_a_follow_may_name_a_written_label() {
        let tests = vec![
            json!({ "test": "COUNT", "in": { "region": "AROUND",
                    "of": { "place": "START" }, "through": "across" } }),
            json!({ "test": "COUNT", "in": { "region": "FOLLOW",
                    "from": { "place": "START" }, "steps": 1, "through": "across" } }),
        ];
        let with = game(
            json!([{ "from": "a", "to": "b", "label": "across" }]),
            "across",
            tests.clone(),
        );
        assert!(read(&with).is_ok(), "{:?}", read(&with).err());
        let without = game(json!([["a", "b"]]), "across", tests);
        let message = refused(&without);
        assert!(
            message.contains("across"),
            "the label no map has is named: {message}"
        );
        assert!(matches!(
            read(&without),
            Err(GameSpecError::UnknownLabel { .. })
        ));
    }

    #[test]
    fn a_follow_over_a_label_that_leads_to_several_places_is_refused() {
        let follow = vec![json!({ "test": "COUNT", "in": { "region": "FOLLOW",
            "from": { "place": "START" }, "steps": 1, "through": "across" } })];
        let spec = game(
            json!([
                { "from": "a", "to": "b", "label": "across" },
                { "from": "a", "to": "c", "label": "across" }
            ]),
            "across",
            follow,
        );
        assert!(matches!(read(&spec), Err(GameSpecError::InvalidStep(_))));
    }

    #[test]
    fn an_empty_a_long_or_a_derived_label_is_refused() {
        let long = "x".repeat(65);
        for label in [
            "",
            long.as_str(),
            "ends",
            "paths",
            "corners",
            "areas",
            "sides",
        ] {
            let spec = plain(json!([{ "from": "a", "to": "b", "label": label }]));
            let message = refused(&spec);
            assert!(
                matches!(read(&spec), Err(GameSpecError::InvalidMap(_))),
                "{label:?}: {message}"
            );
            if !label.is_empty() && label.len() < 65 {
                assert!(message.contains(label), "names the label: {message}");
            }
        }
        // `next` is allowed, and means the unlabelled form.
        let spec = read(&plain(json!([{ "from": "a", "to": "b", "label": "next" }])))
            .expect("next is writable");
        assert_eq!(map(&spec).one_way.len(), 1);
        assert!(map(&spec).labelled.is_empty());
    }

    #[test]
    fn a_labelled_link_joins_points() {
        let mut spec = plain(json!([]));
        spec["board"]["places"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "name": "ab", "kind": "path", "ends": ["a", "b"] }));
        spec["board"]["links"] = json!([{ "from": "a", "to": "ab", "label": "across" }]);
        let message = refused(&spec);
        assert!(
            message.contains("ab") && message.contains("not a point"),
            "{message}"
        );
    }

    #[test]
    fn a_labelled_link_round_trips() {
        let spec = read(&plain(json!([
            { "from": "a", "to": "b", "label": "across" },
            { "between": ["b", "c"], "label": "pair" },
            ["a", "c"]
        ])))
        .expect("reads");
        let again = read(&serde_json::to_value(&spec).expect("a spec serializes"))
            .expect("what was written reads again");
        assert_eq!(map(&spec), map(&again));
    }

    #[test]
    fn an_unknown_key_on_a_labelled_link_is_refused() {
        for link in [
            json!({ "from": "a", "to": "b", "label": "x", "bogus": 1 }),
            json!({ "between": ["a", "b"], "label": "x", "bogus": 1 }),
        ] {
            // An untagged enum names no field in its refusal; the link is refused all the same.
            let message = refused(&plain(json!([link])));
            assert!(
                message.contains("RawLink") || message.contains("link"),
                "{message}"
            );
        }
    }

    #[test]
    fn a_between_link_needs_a_label() {
        assert!(read(&plain(json!([{ "between": ["a", "b"] }]))).is_err());
    }

    #[test]
    fn more_than_sixty_four_labels_are_refused() {
        let links = |count: usize| {
            Value::Array(
                (0..count)
                    .map(|n| json!({ "from": "a", "to": "b", "label": format!("l{n}") }))
                    .collect(),
            )
        };
        assert!(read(&plain(links(64))).is_ok(), "64 labels are fine");
        let message = refused(&plain(links(65)));
        assert!(
            message.contains("64") && message.contains("labels"),
            "{message}"
        );
    }

    #[test]
    fn adjacent_and_orthogonal_are_no_labels_a_link_may_take() {
        for label in ["adjacent", "orthogonal"] {
            let message = refused(&plain(json!([{ "from": "a", "to": "b", "label": label }])));
            assert!(message.contains(label), "{message}");
        }
    }

    #[test]
    fn the_empty_label_refusal_names_the_link() {
        let message = refused(&plain(json!([{ "from": "a", "to": "b", "label": "" }])));
        assert!(
            message.contains("`a`") && message.contains("`b`"),
            "{message}"
        );
    }

    /// Each label leads where it was written, whether it sorts before or after the labels
    /// every map has: `a` sorts before `areas`, `zzz` after `sides`.
    #[test]
    fn every_label_follows_its_own_links() {
        let piece = |code: &str, label: &str| {
            json!({ "code": code, "name": code, "moves": [{ "id": 0, "step": label, "do": [
                { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
            ] }] })
        };
        let mut spec = plain(json!([
            { "from": "a", "to": "b" },
            { "from": "a", "to": "c", "label": "a" },
            { "from": "a", "to": "d", "label": "zzz" }
        ]));
        spec["board"]["places"]
            .as_array_mut()
            .unwrap()
            .push(point("d", [3.0, 0.0]));
        spec["pieces"] = json!([
            piece("ON_NEXT", "next"),
            piece("ON_A", "a"),
            piece("ON_ZZZ", "zzz")
        ]);
        for (code, expected) in [("ON_NEXT", "b"), ("ON_A", "c"), ("ON_ZZZ", "d")] {
            let mut game = crate::logic::Game::from_spec(read(&spec).expect("reads"));
            game.state.pieces.clear();
            game.state.pieces.insert(
                crate::shared::Position::new("a"),
                crate::logic::Piece::new(code.to_string(), "A".to_string()),
            );
            let targets =
                crate::tests::support::moves_from(&mut game, crate::shared::Position::new("a"));
            assert_eq!(
                targets,
                vec![crate::shared::Position::new(expected)],
                "{code}"
            );
        }
    }
}
