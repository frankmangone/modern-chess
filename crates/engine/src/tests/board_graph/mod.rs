//! A grid board compiles into a graph: one node per square, named by its coordinates, one
//! labelled edge table per vector the rules use, and the `orthogonal` and `adjacent`
//! neighbourhoods.
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::logic::{Board, Labels};
    use crate::shared::{Coordinates, Position};
    use crate::specs::{BoardShape, BoardSpec};

    fn board(dimensions: Vec<u8>, disabled: &[Coordinates], labels: &Labels) -> Board {
        Board::new(
            BoardSpec {
                dimensions,
                disabled_positions: disabled.iter().cloned().collect::<HashSet<_>>(),
                shape: BoardShape::Square,
                map: None,
            },
            labels,
        )
    }

    fn names(board: &Board, nodes: &[u32]) -> Vec<Position> {
        nodes
            .iter()
            .map(|&node| board.name(node).unwrap().clone())
            .collect()
    }

    #[test]
    fn names_grid_squares_by_their_coordinates_in_board_order() {
        let board = board(vec![3, 2], &[vec![1, 1]], &Labels::default());
        assert_eq!(
            board.all_positions(),
            vec![pos![0, 0], pos![0, 1], pos![1, 0], pos![2, 0], pos![2, 1]]
        );
        assert_eq!(board.node(&pos![1, 1]), None);
        assert_eq!(board.coordinates(&pos![2, 1]), Some(&[2u8, 1][..]));
    }

    #[test]
    fn follows_a_label_and_stops_at_the_edge_and_at_holes() {
        let mut labels = Labels::default();
        let right = labels.intern(&vec![1, 0]);
        let board = board(vec![3, 2], &[vec![1, 1]], &labels);
        assert_eq!(board.step(&pos![0, 0], right.label), Some(&pos![1, 0]));
        assert_eq!(board.step(&pos![2, 0], right.label), None);
        assert_eq!(board.step(&pos![0, 1], right.label), None);
    }

    #[test]
    fn a_player_label_is_the_vector_turned_by_their_orientation() {
        let mut labels = Labels::default();
        let turned = labels.for_player(&vec![vec![-1, 0], vec![0, -1]], &vec![0, 1]);
        let down = labels.intern(&vec![0, -1]);
        assert_eq!(turned, down);
        assert_eq!(turned.unit, down.label);
        let long = labels.intern(&vec![0, -3]);
        assert_eq!(long.unit, down.label);
    }

    #[test]
    fn orthogonal_and_adjacent_neighbourhoods_on_a_grid() {
        let square = board(vec![3, 3], &[], &Labels::default());
        let centre = square.node(&pos![1, 1]).unwrap();
        assert_eq!(
            names(&square, &square.orthogonal(centre)),
            vec![pos![0, 1], pos![2, 1], pos![1, 0], pos![1, 2]]
        );
        assert_eq!(square.adjacent(centre).len(), 8);
        assert_eq!(square.adjacent(square.node(&pos![0, 0]).unwrap()).len(), 3);

        let cube = board(vec![3, 3, 3], &[], &Labels::default());
        assert_eq!(cube.adjacent(cube.node(&pos![1, 1, 1]).unwrap()).len(), 26);
    }

    #[test]
    fn walks_a_straight_line_between_two_squares() {
        let mut labels = Labels::default();
        let right = labels.intern(&vec![1, 0]);
        let board = board(vec![8, 8], &[], &labels);
        let node = |name: Position| board.node(&name).unwrap();
        assert_eq!(
            board
                .path(node(pos![0, 0]), node(pos![3, 0]), right.unit)
                .map(|nodes| names(&board, &nodes)),
            Some(vec![pos![1, 0], pos![2, 0], pos![3, 0]])
        );
        assert_eq!(
            board.path(node(pos![0, 0]), node(pos![3, 1]), right.unit),
            None
        );
        assert_eq!(
            board.path(node(pos![2, 2]), node(pos![2, 2]), right.unit),
            Some(vec![])
        );
    }

    #[test]
    fn builds_and_walks_a_ten_thousand_square_board() {
        let mut labels = Labels::default();
        let mut diagonal = None;
        for x in -1i16..=1 {
            for y in -1i16..=1 {
                if (x, y) != (0, 0) {
                    let offset = labels.intern(&vec![x, y]);
                    if (x, y) == (1, 1) {
                        diagonal = Some(offset.label);
                    }
                }
            }
        }
        let board = board(vec![100, 100], &[], &labels);
        let mut node = board.node(&pos![0, 0]).unwrap();
        for _ in 0..99 {
            node = board.follow(node, diagonal.unwrap()).unwrap();
        }
        assert_eq!(board.name(node), Some(&pos![99, 99]));
        assert_eq!(board.follow(node, diagonal.unwrap()), None);
    }

    #[test]
    fn grid_names_are_written_as_coordinates() {
        use serde_json::json;
        assert_eq!(serde_json::to_value(pos![4, 2]).unwrap(), json!([4, 2]));
        assert_eq!(serde_json::to_value(Position::from("")).unwrap(), json!([]));
        assert_eq!(
            serde_json::to_value(Position::from("north")).unwrap(),
            json!("north")
        );
        assert_eq!(
            serde_json::from_value::<Position>(json!([4, 2])).unwrap(),
            Position::from("4,2")
        );
        assert_eq!(
            serde_json::from_value::<Position>(json!("north")).unwrap(),
            Position::from("north")
        );
    }

    #[test]
    fn a_name_that_only_looks_like_coordinates_stays_a_string() {
        assert_eq!(
            serde_json::to_value(Position::from("04,2")).unwrap(),
            serde_json::json!("04,2")
        );
    }

    #[test]
    fn a_large_board_prints_briefly() {
        let board = board(vec![100, 100], &[], &Labels::default());
        let printed = format!("{board:?}");
        assert!(printed.len() < 500, "printed {} characters", printed.len());
    }

    #[test]
    fn an_empty_array_reads_back_as_the_empty_name() {
        assert_eq!(
            serde_json::from_value::<Position>(serde_json::json!([])).unwrap(),
            Position::default()
        );
    }

    #[test]
    fn a_rook_slides_across_the_largest_board() {
        let moves: Vec<_> = [[1, 0], [-1, 0], [0, 1], [0, -1]]
            .iter()
            .enumerate()
            .map(|(id, step)| {
                serde_json::json!({
                    "id": id,
                    "step": step,
                    "actions": [
                        { "state": "EMPTY", "action": "MOVE" },
                        { "state": "ENEMY", "action": "CAPTURE" }
                    ],
                    "repeat": { "loop": true }
                })
            })
            .collect();
        let spec = serde_json::json!({
            "name": "WIDE",
            "board": { "dimensions": [100, 100] },
            "leader": [],
            "turns": { "order": ["WHITE", "BLACK"] },
            "players": [
                { "name": "WHITE", "direction": [[1, 0], [0, 1]],
                  "starting_positions": [{ "piece": "ROOK", "positions": [[0, 0]] }] },
                { "name": "BLACK", "direction": [[-1, 0], [0, -1]],
                  "starting_positions": [{ "piece": "ROOK", "positions": [[99, 99]] }] }
            ],
            "pieces": [{
                "code": "ROOK",
                "name": "Rook",
                "moves": moves
            }]
        });
        let mut game = crate::specs::parse_game_spec_str(&spec.to_string())
            .map(crate::logic::Game::from_spec)
            .expect("the spec should load");
        game.transition(crate::logic::GameTransition::CalculateMoves {
            position: pos![0, 0],
        })
        .unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();
        assert_eq!(moves.len(), 198);
        assert!(moves.contains_key(&pos![99, 0]) && moves.contains_key(&pos![0, 99]));

        game.transition(crate::logic::GameTransition::ExecuteMove {
            position: pos![0, 99],
        })
        .unwrap();
        game.transition(crate::logic::GameTransition::CalculateMoves {
            position: pos![99, 99],
        })
        .unwrap();
        let replies = game.state.available_moves.as_ref().unwrap();
        assert_eq!(replies.len(), 198);
        assert_eq!(
            replies.get(&pos![0, 99]).map(|e| e.action.as_str()),
            Some("CAPTURE")
        );
    }
}
