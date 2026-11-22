use modern_chess_session::session::Session;
use modern_chess_session::view::{NodeView, PieceView, Status};

const CHESS: &str = include_str!("../../engine/specs/chess.json");
const SHOGI: &str = include_str!("../../engine/specs/shogi.json");
const MARCH: &str = include_str!("../../../apps/docs/examples/march.json");

#[test]
fn view_of_chess_start() {
    let session = Session::new(CHESS).unwrap();
    let view = session.view();
    assert_eq!(view.nodes.len(), 64);
    assert_eq!(view.pieces.len(), 32);
    assert_eq!(view.turn, "WHITE");
    assert_eq!(view.status, Status::Playing);
    assert_eq!(view.selected, None);
    assert!(view.destinations.is_empty());
    assert_eq!(view.promotion, None);
    assert!(!view.hands);
}

#[test]
fn a_disabled_cell_is_not_a_node() {
    let spec = MARCH.replacen(
        "\"dimensions\": [3, 5] }",
        "\"dimensions\": [3, 5], \"disabled_positions\": [[1, 2]] }",
        1,
    );
    assert_ne!(spec, MARCH);
    let view = Session::new(&spec).unwrap().view();
    assert_eq!(view.nodes.len(), 14);
    assert!(!view.nodes.iter().any(|n| n.name == "1,2"));
}

#[test]
fn the_view_lists_every_node_with_where_to_draw_it() {
    let view = Session::new(CHESS).unwrap().view();
    assert_eq!(view.nodes.len(), 64);
    assert_eq!(
        view.nodes[0],
        NodeView {
            name: "0,0".into(),
            at: [0.0, 0.0],
            kind: None,
            ends: None,
            around: None
        }
    );
    assert!(view.nodes.contains(&NodeView {
        name: "4,2".into(),
        at: [4.0, 2.0],
        kind: None,
        ends: None,
        around: None
    }));
}

#[test]
fn pieces_and_highlights_are_named() {
    let mut s = Session::new(CHESS).unwrap();
    s.select("4,1").unwrap();
    let view = s.view();
    assert_eq!(view.selected.as_deref(), Some("4,1"));
    assert_eq!(
        view.destinations,
        vec!["4,2".to_string(), "4,3".to_string()]
    );
    assert!(view.pieces.contains(&PieceView {
        position: "4,0".into(),
        code: "KING".into(),
        player: "WHITE".into()
    }));
    assert!(view.zones["REACH_END"]["WHITE"].contains(&"0,7".to_string()));
}

#[test]
fn lists_of_names_are_in_board_order() {
    // "2,0" comes before "10,0", which a sort of the text would get wrong.
    let view = Session::new(GLINSKI).unwrap().view();
    let positions: Vec<&str> = view.pieces.iter().map(|p| p.position.as_str()).collect();
    let mut sorted = positions.clone();
    sorted.sort_by_key(|name| {
        name.split(',')
            .map(|n| n.parse::<u8>().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(positions, sorted);
    let names: Vec<&str> = view.nodes.iter().map(|n| n.name.as_str()).collect();
    assert_eq!(names.len(), 91);
    assert!(names.iter().position(|n| *n == "2,5") < names.iter().position(|n| *n == "10,5"));
}

#[test]
fn view_reports_zones() {
    let view = Session::new(CHESS).unwrap().view();
    let zone = view.zones.get("REACH_END").expect("REACH_END zone present");
    assert!(!zone.get("WHITE").expect("WHITE squares").is_empty());
}

#[test]
fn view_flags_hand_games() {
    assert!(Session::new(SHOGI).unwrap().view().hands);
}

#[test]
fn new_rejects_bad_json() {
    assert!(Session::new("{").is_err());
}

#[test]
fn new_rejects_non_2d_board() {
    // The engine's direction matrices are 2x2, so a 3-D board cannot be described at all.
    let spec = MARCH.replacen("[3, 5]", "[3, 5, 2]", 1);
    assert_ne!(spec, MARCH);
    assert!(Session::new(&spec).is_err());
}

#[test]
fn reset_restores_start() {
    let mut session = Session::new(CHESS).unwrap();
    let before = serde_json::to_string(&session.view()).unwrap();
    session.reset();
    let after = serde_json::to_string(&session.view()).unwrap();
    assert_eq!(before, after);
}

#[test]
fn material_that_would_overflow_is_capped_instead_of_panicking() {
    let mut spec: serde_json::Value =
        serde_json::from_str(include_str!("../../engine/specs/chess.json")).unwrap();
    for piece in spec["pieces"].as_array_mut().unwrap() {
        piece["value"] = serde_json::json!(u32::MAX);
    }
    let session = Session::new(&spec.to_string()).unwrap();

    let view = session.view();

    assert_eq!(view.material["WHITE"], u32::MAX);
}

const GLINSKI: &str = include_str!("../../engine/specs/glinski.json");
const HEX: &str = include_str!("../../engine/specs/hex.json");

fn view_json(spec: &str) -> serde_json::Value {
    serde_json::to_value(Session::new(spec).unwrap().view()).unwrap()
}

#[test]
fn a_square_board_says_square() {
    let view = view_json(CHESS);
    assert_eq!(view["shape"], "square");
    assert!(view.get("layout").is_none());
}

#[test]
fn a_hexagon_board_says_hex_and_lists_its_corners() {
    let view = view_json(GLINSKI);
    assert_eq!(view["shape"], "hex");
    assert_eq!(view["layout"], "hexagon");
    assert_eq!(view["nodes"].as_array().unwrap().len(), 91);
}

#[test]
fn a_rhombus_board_says_rhombus() {
    let view = view_json(HEX);
    assert_eq!(view["shape"], "hex");
    assert_eq!(view["layout"], "rhombus");
    assert_eq!(view["nodes"].as_array().unwrap().len(), 121);
}
