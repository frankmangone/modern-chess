//! Written maps: places keep their order, links are worked out when the spec is read, and each
//! refusal names the place at fault.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::Game;
    use crate::specs::{
        parse_game_spec_str, BoardSpec, GameSpec, GameSpecError, MapSpec, PlaceKind,
    };
    use crate::tests::support::place_action;

    fn point(name: &str, at: [f64; 2]) -> Value {
        json!({ "name": name, "at": at })
    }

    fn path(name: &str, a: &str, b: &str) -> Value {
        json!({ "name": name, "kind": "path", "ends": [a, b] })
    }

    fn area(name: &str, around: &[&str]) -> Value {
        json!({ "name": name, "kind": "area", "around": around })
    }

    /// Four points in a square, the paths along its sides, the area `f` inside them and the
    /// link `a`-`c`. With `g`: a point `e` and the area `g` around `b e c`.
    fn places(with_g: bool) -> Vec<Value> {
        let mut places = vec![
            point("a", [0.0, 0.0]),
            point("b", [2.0, 0.0]),
            point("c", [2.0, 2.0]),
            point("d", [0.0, 2.0]),
        ];
        if with_g {
            places.push(point("e", [4.0, 1.0]));
        }
        places.extend([
            path("ab", "a", "b"),
            path("bc", "b", "c"),
            path("cd", "c", "d"),
            path("da", "d", "a"),
            area("f", &["a", "b", "c", "d"]),
        ]);
        if with_g {
            places.push(area("g", &["b", "e", "c"]));
        }
        places
    }

    fn game(board: Value) -> String {
        json!({
            "name": "MAP TEST",
            "board": board,
            "turns": { "order": ["A", "B"] },
            "players": [
                { "name": "A", "starting_positions": [] },
                { "name": "B", "starting_positions": [] }
            ],
            "pieces": [{ "code": "STONE", "name": "stone", "moves": [] }],
            "actions": [place_action("STONE")]
        })
        .to_string()
    }

    fn board(places: Vec<Value>) -> Value {
        json!({ "shape": "map", "places": places, "links": [["a", "c"]] })
    }

    fn read(board: Value) -> Result<GameSpec, GameSpecError> {
        parse_game_spec_str(&game(board))
    }

    fn map_of(spec: &GameSpec) -> &MapSpec {
        spec.board.map.as_deref().expect("a map board")
    }

    fn written(with_g: bool) -> GameSpec {
        read(board(places(with_g))).expect("a valid map")
    }

    /// The names a place links to under `label`.
    fn linked<'a>(map: &'a MapSpec, from: &str, label: &str) -> Vec<&'a str> {
        let from = map.index(from).expect("a place");
        map.links[label][from]
            .iter()
            .map(|&to| map.places[to].name.as_str())
            .collect()
    }

    #[test]
    fn places_keep_board_order_and_get_at() {
        let spec = written(false);
        let map = map_of(&spec);
        let names: Vec<&str> = map.places.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["a", "b", "c", "d", "ab", "bc", "cd", "da", "f"]);
        assert_eq!(map.places[map.index("ab").unwrap()].at, [1.0, 0.0]);
        assert_eq!(map.places[map.index("f").unwrap()].at, [1.0, 1.0]);
        assert_eq!(map.places[map.index("c").unwrap()].at, [2.0, 2.0]);
        assert_eq!(map.places[map.index("f").unwrap()].kind, PlaceKind::Area);
        assert_eq!(map.index("nowhere"), None);
        assert_eq!(map.lines, [["a".to_string(), "c".to_string()]]);
    }

    #[test]
    fn a_board_that_is_a_map_has_no_grid() {
        let spec = written(false);
        assert!(spec.board.dimensions.is_empty());
        assert!(spec.board.disabled_positions.is_empty());
    }

    #[test]
    fn every_label_has_a_list_for_every_place() {
        let spec = read(json!({ "shape": "map", "places": [point("a", [0.0, 0.0])] }))
            .expect("one point is a map");
        let map = map_of(&spec);
        let labels: Vec<&str> = map.links.keys().map(String::as_str).collect();
        assert_eq!(
            labels,
            ["areas", "corners", "ends", "next", "paths", "sides"]
        );
        assert!(map
            .links
            .values()
            .all(|lists| lists == &vec![Vec::<usize>::new()]));
    }

    #[test]
    fn implied_links() {
        let spec = written(true);
        let map = map_of(&spec);
        assert_eq!(linked(map, "a", "next"), ["b", "c", "d"]);
        assert_eq!(linked(map, "c", "next"), ["a", "b", "d"]);
        assert_eq!(linked(map, "ab", "next"), ["bc", "da"]);
        assert_eq!(linked(map, "ab", "ends"), ["a", "b"]);
        assert_eq!(linked(map, "a", "paths"), ["ab", "da"]);
        assert_eq!(linked(map, "f", "corners"), ["a", "b", "c", "d"]);
        assert_eq!(linked(map, "g", "corners"), ["b", "c", "e"]);
        assert_eq!(linked(map, "a", "areas"), ["f"]);
        assert_eq!(linked(map, "b", "areas"), ["f", "g"]);
        assert_eq!(linked(map, "ab", "areas"), ["f"]);
        assert_eq!(linked(map, "bc", "areas"), ["f", "g"]);
        assert_eq!(linked(map, "g", "sides"), ["bc"]);
        assert_eq!(linked(map, "f", "sides"), ["ab", "bc", "cd", "da"]);
        assert_eq!(linked(map, "f", "next"), ["g"]);
        assert_eq!(linked(map, "g", "next"), ["f"]);
        assert!(linked(map, "a", "ends").is_empty());
        assert!(linked(map, "ab", "corners").is_empty());
    }

    fn refused(board: Value, name: &str) {
        match read(board) {
            Err(GameSpecError::InvalidMap(message)) => {
                assert!(message.contains(name), "`{message}` should name `{name}`");
            }
            other => panic!("expected an InvalidMap naming `{name}`, got {other:?}"),
        }
    }

    #[test]
    fn a_point_must_lie_within_the_drawing_bound() {
        let bound = 1_000_000.0;
        let lone = |at: [f64; 2]| {
            board(vec![
                point("far", at),
                point("a", [0.0, 0.0]),
                point("c", [1.0, 0.0]),
            ])
        };

        for at in [[bound, -bound], [-bound, bound], [0.0, bound]] {
            let spec = read(lone(at)).expect("a point on the bound is read");
            assert_eq!(map_of(&spec).places[0].at, at);
        }
        for at in [
            [bound + 1.0, 0.0],
            [0.0, -bound - 1.0],
            [1e308, 0.0],
            [1.7e308, 0.0],
        ] {
            refused(lone(at), "far");
        }
        // Two far points could still overflow a path's midpoint, so each is bounded alone.
        refused(
            board(vec![
                point("p", [1.7e308, 0.0]),
                point("q", [1.7e308, 0.0]),
                path("pq", "p", "q"),
                point("a", [0.0, 0.0]),
                point("c", [1.0, 0.0]),
            ]),
            "p",
        );
    }

    fn with(extra: Vec<Value>) -> Value {
        let mut all = places(false);
        all.extend(extra);
        board(all)
    }

    #[test]
    fn refusals() {
        refused(with(vec![point("a", [5.0, 5.0])]), "a");
        refused(with(vec![point("", [5.0, 5.0])]), "name");
        let long = "x".repeat(65);
        refused(with(vec![point(&long, [5.0, 5.0])]), &long);
        refused(with(vec![path("zz", "a", "nowhere")]), "nowhere");
        refused(with(vec![path("zz", "ab", "a")]), "ab");
        refused(with(vec![path("zz", "a", "a")]), "zz");
        refused(with(vec![area("zz", &["a", "b"])]), "zz");
        refused(with(vec![area("zz", &["a", "b", "f"])]), "f");
        refused(with(vec![json!({ "name": "lonely" })]), "lonely");
        refused(board(vec![]), "places");
        refused(
            json!({ "shape": "map", "places": places(false), "links": [["a", "nowhere"]] }),
            "nowhere",
        );
        refused(
            json!({
                "shape": "map",
                "places": places(false),
                "disabled_positions": [[0, 0]]
            }),
            "disabled_positions",
        );
    }

    #[test]
    fn a_name_of_64_bytes_is_fine() {
        let long = "x".repeat(64);
        assert!(read(with(vec![point(&long, [5.0, 5.0])])).is_ok());
    }

    #[test]
    fn a_written_link_joins_points_only() {
        let linked = json!({ "shape": "map", "places": places(false), "links": [["a", "ab"]] });
        refused(linked, "ab");
        let itself = json!({ "shape": "map", "places": places(false), "links": [["a", "a"]] });
        refused(itself, "a");
    }

    #[test]
    fn a_map_of_more_than_10000_places_is_refused() {
        let many: Vec<Value> = (0..10_001)
            .map(|i| point(&format!("p{i}"), [f64::from(i), 0.0]))
            .collect();
        refused(json!({ "shape": "map", "places": many }), "10000");
    }

    #[test]
    fn a_map_game_builds_without_a_grid() {
        let spec = written(false);
        let game = Game::from_spec(spec);
        let names: Vec<String> = game
            .board
            .all_positions()
            .iter()
            .map(|position| position.to_string())
            .collect();
        assert_eq!(names, ["a", "b", "c", "d", "ab", "bc", "cd", "da", "f"]);
    }

    #[test]
    fn round_trip() {
        let spec = written(true);
        let text = serde_json::to_string(&spec.board).expect("a board writes");
        let again: BoardSpec = serde_json::from_str(&text).expect("it reads back");
        let (first, second) = (spec.board.map.expect("map"), again.map.expect("map"));
        assert_eq!(first, second);
        let written: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(written["shape"], "map");
        assert_eq!(written["places"].as_array().map(Vec::len), Some(11));
        assert_eq!(written["places"][0], point("a", [0.0, 0.0]));
        assert_eq!(
            written["places"][4],
            json!({ "name": "e", "at": [4.0, 1.0] })
        );
        assert_eq!(written["places"][5], path("ab", "a", "b"));
        assert_eq!(written["links"], json!([["a", "c"]]));
    }

    /// Two points joined by `count` parallel paths.
    fn parallel(count: usize) -> Value {
        let mut all = vec![point("hub", [0.0, 0.0]), point("far", [1.0, 0.0])];
        all.extend((0..count).map(|i| path(&format!("p{i}"), "hub", "far")));
        json!({ "shape": "map", "places": all })
    }

    #[test]
    fn many_parallel_paths_are_refused_quickly_naming_the_point() {
        let started = std::time::Instant::now();
        match read(parallel(9_998)) {
            Err(GameSpecError::InvalidMap(message)) => {
                assert!(message.contains("hub"), "`{message}` should name `hub`")
            }
            other => panic!("expected an InvalidMap, got {other:?}"),
        }
        assert!(started.elapsed() < std::time::Duration::from_secs(5));
        refused(parallel(1_001), "hub");
    }

    #[test]
    fn parallel_paths_just_under_the_budget_are_accepted() {
        // Both points count: 2 x 700 x 699 = 978,600 entries.
        let spec = read(parallel(700)).expect("700 parallel paths are within the budget");
        let map = map_of(&spec);
        assert_eq!(linked(map, "p0", "next").len(), 699);
        assert!(read(parallel(710)).is_err());
    }

    #[test]
    fn many_duplicate_areas_are_refused_naming_the_side() {
        let mut all = vec![
            point("a", [0.0, 0.0]),
            point("b", [1.0, 0.0]),
            point("c", [0.0, 1.0]),
        ];
        all.extend((0..1_100).map(|i| area(&format!("t{i}"), &["a", "b", "c"])));
        refused(
            json!({ "shape": "map", "places": all }),
            "areas along the side",
        );
    }
}
