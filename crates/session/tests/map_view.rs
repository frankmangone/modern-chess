use modern_chess_session::session::Session;
use serde_json::{json, Value};

const CHESS: &str = include_str!("../../engine/specs/chess.json");

/// Task 2's square map: points a b c d, the paths along the sides, the area f inside them, and
/// the written link a-c. A walker on `a` steps along `next` to an empty place.
fn square_map_spec() -> String {
    json!({
        "name": "MAP VIEW",
        "board": {
            "shape": "map",
            "places": [
                { "name": "a", "at": [0.0, 0.0] },
                { "name": "b", "at": [1.0, 0.0] },
                { "name": "c", "at": [1.0, 1.0] },
                { "name": "d", "at": [0.0, 1.0] },
                { "name": "ab", "kind": "path", "ends": ["a", "b"] },
                { "name": "bc", "kind": "path", "ends": ["b", "c"] },
                { "name": "cd", "kind": "path", "ends": ["c", "d"] },
                { "name": "da", "kind": "path", "ends": ["d", "a"] },
                { "name": "f", "kind": "area", "around": ["a", "b", "c", "d"] }
            ],
            "links": [["a", "c"]]
        },
        "leader": [],
        "turns": { "order": ["A", "B"] },
        "players": [
            { "name": "A", "starting_positions": [{ "piece": "WALKER", "positions": ["a"] }] },
            { "name": "B", "starting_positions": [] }
        ],
        "pieces": [{
            "code": "WALKER",
            "name": "walker",
            "moves": [{
                "id": 0,
                "step": "next",
                "actions": [{ "state": "EMPTY", "action": "MOVE" }]
            }]
        }]
    })
    .to_string()
}

fn view_json(spec: &str) -> Value {
    let view = Session::new(spec).unwrap().view();
    serde_json::to_value(view).unwrap()
}

#[test]
fn a_map_view_lists_every_place_with_its_kind() {
    let view = view_json(&square_map_spec());
    assert_eq!(view["shape"], "map");
    assert!(view.get("layout").is_none());
    assert_eq!(
        view["nodes"],
        json!([
            { "name": "a", "at": [0, 0], "kind": "point" },
            { "name": "b", "at": [1, 0], "kind": "point" },
            { "name": "c", "at": [1, 1], "kind": "point" },
            { "name": "d", "at": [0, 1], "kind": "point" },
            { "name": "ab", "at": [0.5, 0], "kind": "path", "ends": ["a", "b"] },
            { "name": "bc", "at": [1, 0.5], "kind": "path", "ends": ["b", "c"] },
            { "name": "cd", "at": [0.5, 1], "kind": "path", "ends": ["c", "d"] },
            { "name": "da", "at": [0, 0.5], "kind": "path", "ends": ["d", "a"] },
            { "name": "f", "at": [0.5, 0.5], "kind": "area", "around": ["a", "b", "c", "d"] }
        ])
    );
}

#[test]
fn a_map_view_lists_the_written_lines() {
    let view = view_json(&square_map_spec());
    assert_eq!(view["lines"], json!([["a", "c"]]));
}

#[test]
fn selecting_a_piece_on_a_map_gives_destinations_by_name() {
    let mut session = Session::new(&square_map_spec()).unwrap();
    session.select("a").unwrap();
    let view = session.view();
    assert_eq!(view.selected.as_deref(), Some("a"));
    assert_eq!(view.destinations, vec!["b", "c", "d"]);
}

#[test]
fn grid_view_unchanged() {
    let view = view_json(CHESS);
    assert_eq!(view["shape"], "square");
    let nodes = view["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 64);
    assert_eq!(nodes[0], json!({ "name": "0,0", "at": [0, 0] }));
    let session = Session::new(CHESS).unwrap();
    let nodes_text = serde_json::to_string(&session.view().nodes).unwrap();
    assert!(
        nodes_text.starts_with(r#"[{"name":"0,0","at":[0,0]},"#),
        "{nodes_text}"
    );
    assert!(
        nodes_text.contains(r#"{"name":"4,2","at":[4,2]}"#),
        "{nodes_text}"
    );
    for node in nodes {
        for key in ["kind", "ends", "around"] {
            assert!(node.get(key).is_none(), "grid node has `{key}`");
        }
    }
    assert!(view.get("lines").is_none());
}
