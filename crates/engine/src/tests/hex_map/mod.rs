//! Hex maps: tiles, corners and sides generated from a hex layout, with the links worked out by
//! the same code as a written map's.
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use serde_json::{json, Value};

    use crate::specs::game::hex_map::hex_map;
    use crate::specs::{
        parse_game_spec_str, BoardShape, BoardSpec, GameSpec, GameSpecError, HexLayout, MapSpec,
        PlaceKind,
    };

    const SQRT3: f64 = 1.732_050_807_568_877_2;

    fn none() -> HashSet<Vec<u8>> {
        HashSet::new()
    }

    fn catan() -> MapSpec {
        hex_map(HexLayout::Hexagon, 3, &none()).expect("a hexagon of size 3")
    }

    fn count(map: &MapSpec, kind: PlaceKind) -> usize {
        map.places.iter().filter(|place| place.kind == kind).count()
    }

    fn place<'a>(map: &'a MapSpec, name: &str) -> &'a crate::specs::PlaceSpec {
        let index = map
            .index(name)
            .unwrap_or_else(|| panic!("no place `{name}`"));
        &map.places[index]
    }

    fn linked<'a>(map: &'a MapSpec, from: &str, label: &str) -> Vec<&'a str> {
        let from = map.index(from).expect("a place");
        map.links[label][from]
            .iter()
            .map(|&to| map.places[to].name.as_str())
            .collect()
    }

    fn near(found: [f64; 2], expected: [f64; 2]) -> bool {
        (found[0] - expected[0]).abs() < 1e-9 && (found[1] - expected[1]).abs() < 1e-9
    }

    #[test]
    fn catan_counts() {
        let map = catan();
        assert_eq!(count(&map, PlaceKind::Area), 19);
        assert_eq!(count(&map, PlaceKind::Point), 54);
        assert_eq!(count(&map, PlaceKind::Path), 72);

        let rhombus = hex_map(HexLayout::Rhombus, 2, &none()).expect("a rhombus of size 2");
        assert_eq!(count(&rhombus, PlaceKind::Area), 4);
        assert_eq!(count(&rhombus, PlaceKind::Point), 16);
        assert_eq!(count(&rhombus, PlaceKind::Path), 19);
    }

    #[test]
    fn places_come_tiles_then_corners_then_sides_each_sorted() {
        let map = catan();
        let kinds: Vec<PlaceKind> = map.places.iter().map(|place| place.kind).collect();
        let mut expected = vec![PlaceKind::Area; 19];
        expected.extend(vec![PlaceKind::Point; 54]);
        expected.extend(vec![PlaceKind::Path; 72]);
        assert_eq!(kinds, expected);
        let names: Vec<&str> = map.places.iter().map(|place| place.name.as_str()).collect();
        assert_eq!(&names[..3], ["0,0", "0,1", "0,2"]);
        // Numeric, not alphabetic: "-1,-1" comes before "0,0" and "10,0" would follow "9,0".
        assert_eq!(names[19], "-1,-1|-1,0|0,0");
        assert_eq!(names[19 + 54], "-1,-1|0,0");
    }

    #[test]
    fn names() {
        let map = catan();
        let centre = place(&map, "2,2");
        assert_eq!(centre.kind, PlaceKind::Area);
        assert_eq!(
            centre.around,
            [
                "2,2|2,3|3,3",
                "2,2|3,2|3,3",
                "2,1|2,2|3,2",
                "1,1|2,1|2,2",
                "1,1|1,2|2,2",
                "1,2|2,2|2,3",
            ]
        );
        assert_eq!(place(&map, "1,1|1,2|2,2").kind, PlaceKind::Point);
        assert_eq!(place(&map, "2,2|2,3").kind, PlaceKind::Path);
        assert_eq!(
            place(&map, "2,2|2,3").ends,
            Some(["1,2|2,2|2,3".to_string(), "2,2|2,3|3,3".to_string()])
        );

        // The edge tile (0,0) has corners made of tiles that are off the board.
        let edge = place(&map, "0,0");
        assert!(edge.around.iter().any(|name| name.starts_with("-1,")));
        assert!(map.index("-1,0|0,0").is_some());
    }

    #[test]
    fn links_fit() {
        let map = catan();
        for tile in map.places.iter().filter(|p| p.kind == PlaceKind::Area) {
            assert_eq!(
                linked(&map, &tile.name, "corners").len(),
                6,
                "{}",
                tile.name
            );
        }
        for corner in map.places.iter().filter(|p| p.kind == PlaceKind::Point) {
            let areas = linked(&map, &corner.name, "areas").len();
            assert!((1..=3).contains(&areas), "{}", corner.name);
            let next = linked(&map, &corner.name, "next").len();
            assert!((2..=3).contains(&next), "{}", corner.name);
        }
        for side in map.places.iter().filter(|p| p.kind == PlaceKind::Path) {
            assert_eq!(linked(&map, &side.name, "ends").len(), 2, "{}", side.name);
        }
        let mut around_centre = linked(&map, "2,2", "next");
        around_centre.sort_unstable();
        assert_eq!(around_centre, ["1,1", "1,2", "2,1", "2,3", "3,2", "3,3"]);
        assert_eq!(linked(&map, "2,2", "sides").len(), 6);
    }

    /// `hexPoint` in `apps/docs/theme/board-model.js`.
    fn hex_point(rhombus: bool, q: f64, r: f64) -> [f64; 2] {
        let (x, y) = (1.5 * q, SQRT3 * (r - q / 2.0));
        if rhombus {
            [-0.5 * x + SQRT3 / 2.0 * y, -SQRT3 / 2.0 * x - 0.5 * y]
        } else {
            [x, y]
        }
    }

    fn tile_at(rhombus: bool, name: &str) -> [f64; 2] {
        let (q, r) = name.split_once(',').expect("a tile name");
        hex_point(rhombus, q.parse().expect("q"), r.parse().expect("r"))
    }

    fn mean(points: &[[f64; 2]]) -> [f64; 2] {
        let n = points.len() as f64;
        [
            points.iter().map(|p| p[0]).sum::<f64>() / n,
            points.iter().map(|p| p[1]).sum::<f64>() / n,
        ]
    }

    #[test]
    fn at_matches_the_hex_board() {
        for (layout, size, rhombus) in [
            (HexLayout::Hexagon, 3, false),
            (HexLayout::Rhombus, 3, true),
        ] {
            let map = hex_map(layout, size, &none()).expect("a hex map");
            for each in &map.places {
                let tiles: Vec<[f64; 2]> = each
                    .name
                    .split('|')
                    .map(|tile| tile_at(rhombus, tile))
                    .collect();
                let expected = mean(&tiles);
                assert!(
                    near(each.at, expected),
                    "{}: {:?} is not {expected:?}",
                    each.name,
                    each.at
                );
            }
        }
    }

    #[test]
    fn disabled_tiles() {
        let disabled: HashSet<Vec<u8>> = [vec![2, 2]].into_iter().collect();
        let map = hex_map(HexLayout::Hexagon, 3, &disabled).expect("a hexagon with a hole");
        assert_eq!(count(&map, PlaceKind::Area), 18);
        assert!(map.index("2,2").is_none());
        for corner in ["2,2|2,3|3,3", "1,1|1,2|2,2", "2,1|2,2|3,2"] {
            let at = map.index(corner).expect("the corner still exists");
            assert!(!map.links["areas"][at].is_empty(), "{corner}");
        }
    }

    #[test]
    fn a_map_with_no_tiles_is_refused() {
        let all: HashSet<Vec<u8>> = [vec![0, 0]].into_iter().collect();
        assert!(hex_map(HexLayout::Rhombus, 1, &all).is_err());
        assert!(hex_map(HexLayout::Rhombus, 0, &none()).is_err());
        assert!(hex_map(HexLayout::Hexagon, 255, &none()).is_err());
    }

    #[test]
    fn a_map_over_the_place_limit_is_refused_without_building_it() {
        let started = std::time::Instant::now();
        let refused = hex_map(HexLayout::Rhombus, 255, &none());
        assert!(refused.is_err());
        assert!(started.elapsed().as_secs() < 2);
    }

    fn game(board: Value) -> String {
        json!({
            "name": "HEX MAP TEST",
            "board": board,
            "leader": [],
            "turns": { "order": ["A", "B"] },
            "players": [
                { "name": "A", "starting_positions": [] },
                { "name": "B", "starting_positions": [] }
            ],
            "pieces": [{ "code": "STONE", "name": "stone", "unlimited_supply": true, "moves": [] }]
        })
        .to_string()
    }

    fn read(board: Value) -> Result<GameSpec, GameSpecError> {
        parse_game_spec_str(&game(board))
    }

    #[test]
    fn the_board_spec_is_a_map() {
        let spec = read(json!({ "shape": "hex-map", "layout": "hexagon", "size": 3 }))
            .expect("a hex map board");
        assert_eq!(spec.board.shape, BoardShape::Map);
        assert!(spec.board.dimensions.is_empty());
        assert!(spec.board.disabled_positions.is_empty());
        let map = spec.board.map.as_deref().expect("a map");
        assert_eq!(map.places.len(), 19 + 54 + 72);
    }

    #[test]
    fn round_trip() {
        let board = json!({
            "shape": "hex-map", "layout": "rhombus", "size": 3, "disabled_positions": [[1, 1]]
        });
        let spec = read(board.clone()).expect("a hex map board");
        let text = serde_json::to_string(&spec.board).expect("a board writes");
        let written: Value = serde_json::from_str(&text).expect("json");
        assert_eq!(written, board);
        let again: BoardSpec = serde_json::from_str(&text).expect("it reads back");
        assert_eq!(spec.board.map, again.map);
        assert_eq!(
            spec.board
                .map
                .as_deref()
                .map(|map| count(map, PlaceKind::Area)),
            Some(8)
        );

        let plain = read(json!({ "shape": "hex-map", "layout": "hexagon", "size": 2 }))
            .expect("a hex map board");
        let text = serde_json::to_string(&plain.board).expect("a board writes");
        let written: Value = serde_json::from_str(&text).expect("json");
        assert_eq!(
            written,
            json!({
                "shape": "hex-map", "layout": "hexagon", "size": 2, "disabled_positions": []
            })
        );
    }

    #[test]
    fn refusals_are_readable_map_errors() {
        for board in [
            json!({ "shape": "hex-map", "size": 3 }),
            json!({ "shape": "hex-map", "layout": "hexagon" }),
            json!({ "shape": "hex-map", "layout": "hexagon", "size": 0 }),
            json!({ "shape": "hex-map", "layout": "hexagon", "size": 3, "dimensions": [3, 3] }),
            json!({ "shape": "hex-map", "layout": "hexagon", "size": 3, "places": [] }),
        ] {
            assert!(read(board.clone()).is_err(), "{board}");
        }
        match read(json!({
            "shape": "hex-map", "layout": "rhombus", "size": 1, "disabled_positions": [[0, 0]]
        })) {
            Err(GameSpecError::InvalidMap(message)) => assert!(message.contains("no places")),
            other => panic!("expected a map refusal, got {other:?}"),
        }
    }
}
