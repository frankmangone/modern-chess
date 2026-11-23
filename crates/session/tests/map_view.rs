mod support;

use modern_chess_session::session::Session;
use serde_json::{json, Value};

const CHESS: &str = include_str!("../../engine/specs/chess.json");

/// The steps of a move onto an empty place: the piece goes there.
fn onto_an_empty_place() -> Value {
    json!([
        { "test": "NOT", "of": {
            "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }
        } },
        { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
    ])
}

/// The action MOVE: pick a piece of the mover's, then one of its moves.
fn move_action() -> Value {
    json!({ "name": "MOVE", "do": [
        { "choose": "PIECE", "where": { "owner": "ALLY" } },
        { "choose": "MOVE" }
    ] })
}

/// Task 2's square map: points a b c d, the paths along the sides, the area f inside them, and
/// the written link a-c. A walker on `a` steps along `next` to an empty place.
fn square_map_spec() -> String {
    json!({
        "format_version": 1,
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
                "do": onto_an_empty_place()
            }]
        }],
        "actions": [move_action()]
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
    support::at(&mut session, "a").unwrap();
    let view = session.view();
    assert_eq!(view.choice.start.as_ref().map(|p| p.as_str()), Some("a"));
    assert_eq!(support::destinations(&view), vec!["b", "c", "d"]);
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

/// Places named like numbers: the game server resumes from the saved state on every request,
/// and a place must keep its name through that.
fn numbered_places_spec() -> String {
    json!({
        "format_version": 1,
        "name": "NUMBERED",
        "board": {
            "shape": "map",
            "places": [
                { "name": "01", "at": [0, 0] },
                { "name": "02", "at": [1, 0] },
                { "name": "1, 2", "at": [2, 0] }
            ],
            "links": [["01", "02"], ["02", "1, 2"]]
        },
        "turns": { "order": ["A", "B"] },
        "players": [
            { "name": "A", "starting_positions": [{ "piece": "WALKER", "positions": ["01"] }] },
            { "name": "B", "starting_positions": [] }
        ],
        "pieces": [{
            "code": "WALKER",
            "name": "walker",
            "moves": [{
                "id": 0,
                "step": "next",
                "do": onto_an_empty_place()
            }]
        }],
        "actions": [move_action()]
    })
    .to_string()
}

#[test]
fn a_resumed_map_session_keeps_places_named_like_numbers() {
    let spec = numbered_places_spec();
    let live = Session::new(&spec).unwrap();
    let mut resumed = Session::resume(&spec, &live.state(), 1).expect("a saved state resumes");

    let on: Vec<String> = resumed
        .view()
        .pieces
        .iter()
        .map(|piece| piece.position.clone())
        .collect();
    assert_eq!(on, vec!["01"]);

    support::at(&mut resumed, "01").unwrap();
    assert_eq!(support::destinations(&resumed.view()), vec!["02"]);
    support::at(&mut resumed, "02").unwrap();
    assert_eq!(resumed.view().pieces[0].position, "02");
}
