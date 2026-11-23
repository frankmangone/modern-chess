//! Hex boards: the short form compiles to a grid with six neighbours per cell.
#[cfg(test)]
mod tests {
    use crate::shared::IdSet;

    use serde_json::{json, Value};

    use crate::logic::{Board, Game};
    use crate::shared::Position;
    use crate::specs::{parse_game_spec, parse_game_spec_str, BoardShape};
    use crate::tests::support::place_action;

    fn spec(board: Value) -> String {
        json!({
            "format_version": 1,
            "name": "HEX TEST",
            "board": board,
            "turns": { "order": ["A", "B"] },
            "players": [
                { "name": "A", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "B", "direction": [[1, 0], [0, 1]], "starting_positions": [] }
            ],
            "pieces": [{ "code": "STONE", "name": "stone", "moves": [] }],
            "actions": [place_action("STONE")]
        })
        .to_string()
    }

    fn board(board_json: Value) -> Board {
        let game = parse_game_spec_str(&spec(board_json))
            .map(Game::from_spec)
            .expect("a valid hex spec");
        game.board
    }

    fn set(positions: Vec<Position>) -> IdSet<Position> {
        positions.into_iter().collect()
    }

    /// The cells around `position`, through `orthogonal` or `adjacent`.
    fn around(board: &Board, position: Position, adjacent: bool) -> Vec<Position> {
        let node = board.node(&position).expect("the cell is on the board");
        let nodes = if adjacent {
            board.adjacent(node)
        } else {
            board.orthogonal(node)
        };
        nodes
            .into_iter()
            .filter_map(|n| board.name(n).cloned())
            .collect()
    }

    #[test]
    fn a_hexagon_of_side_6_has_91_cells() {
        let board = board(json!({ "shape": "hex", "layout": "hexagon", "size": 6 }));
        assert_eq!(board.all_positions().len(), 91);
        let names = set(board.all_positions());
        assert!(!names.contains(&pos![0, 10]));
        assert!(!names.contains(&pos![10, 0]));
        assert!(names.contains(&pos![5, 5]));
    }

    #[test]
    fn a_rhombus_of_size_11_has_121_cells() {
        let board = board(json!({ "shape": "hex", "layout": "rhombus", "size": 11 }));
        assert_eq!(board.all_positions().len(), 121);
    }

    #[test]
    fn a_hex_cell_has_six_neighbours() {
        let board = board(json!({ "shape": "hex", "layout": "hexagon", "size": 6 }));
        let expected = set(vec![
            pos![5, 6],
            pos![5, 4],
            pos![6, 6],
            pos![6, 5],
            pos![4, 5],
            pos![4, 4],
        ]);
        assert_eq!(set(around(&board, pos![5, 5], false)), expected);
        assert_eq!(set(around(&board, pos![5, 5], true)), expected);
    }

    #[test]
    fn rhombus_corners_have_two_or_three_neighbours() {
        let board = board(json!({ "shape": "hex", "layout": "rhombus", "size": 11 }));
        assert_eq!(around(&board, pos![0, 0], true).len(), 3);
        assert_eq!(around(&board, pos![10, 10], true).len(), 3);
        assert_eq!(around(&board, pos![10, 0], true).len(), 2);
        assert_eq!(around(&board, pos![0, 10], true).len(), 2);
    }

    #[test]
    fn a_hex_spec_with_dimensions_is_refused() {
        let text =
            spec(json!({ "shape": "hex", "layout": "rhombus", "size": 5, "dimensions": [5, 5] }));
        assert!(parse_game_spec_str(&text).is_err());
    }

    #[test]
    fn a_hex_size_of_zero_is_refused() {
        let text = spec(json!({ "shape": "hex", "layout": "rhombus", "size": 0 }));
        let error = parse_game_spec_str(&text)
            .err()
            .expect("refused")
            .to_string();
        assert!(error.contains("`size` of at least 1"), "{error}");
    }

    #[test]
    fn a_square_spec_still_parses_unchanged() {
        let chess = parse_game_spec("./specs/chess.json").expect("chess parses");
        assert_eq!(chess.board.shape, BoardShape::Square);
        assert_eq!(chess.board.dimensions, vec![8, 8]);
    }

    #[test]
    fn disabled_positions_punch_holes_in_a_hexagon() {
        let board = board(json!({
            "shape": "hex", "layout": "hexagon", "size": 6, "disabled_positions": [[5, 5]]
        }));
        assert_eq!(board.all_positions().len(), 90);
    }
}
