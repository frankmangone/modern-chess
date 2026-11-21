//! A grid board compiles into a graph: one node per square, named by its coordinates, one
//! labelled edge table per vector the rules use, and the `orthogonal` and `adjacent`
//! neighbourhoods.
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::logic::{Board, Labels};
    use crate::shared::Position;
    use crate::specs::BoardSpec;

    fn board(dimensions: Vec<u8>, disabled: &[Position], labels: &Labels) -> Board {
        Board::new(
            BoardSpec {
                dimensions,
                disabled_positions: disabled.iter().cloned().collect::<HashSet<_>>(),
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
            vec![vec![0, 0], vec![0, 1], vec![1, 0], vec![2, 0], vec![2, 1]]
        );
        assert_eq!(board.node(&vec![1, 1]), None);
        assert_eq!(board.coordinates(&vec![2, 1]), Some(&[2u8, 1][..]));
    }

    #[test]
    fn follows_a_label_and_stops_at_the_edge_and_at_holes() {
        let mut labels = Labels::default();
        let right = labels.intern(&vec![1, 0]);
        let board = board(vec![3, 2], &[vec![1, 1]], &labels);
        assert_eq!(board.step(&vec![0, 0], right.label), Some(&vec![1, 0]));
        assert_eq!(board.step(&vec![2, 0], right.label), None);
        assert_eq!(board.step(&vec![0, 1], right.label), None);
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
        let centre = square.node(&vec![1, 1]).unwrap();
        assert_eq!(
            names(&square, square.orthogonal(centre)),
            vec![vec![0, 1], vec![2, 1], vec![1, 0], vec![1, 2]]
        );
        assert_eq!(square.adjacent(centre).len(), 8);
        assert_eq!(square.adjacent(square.node(&vec![0, 0]).unwrap()).len(), 3);

        let cube = board(vec![3, 3, 3], &[], &Labels::default());
        assert_eq!(cube.adjacent(cube.node(&vec![1, 1, 1]).unwrap()).len(), 26);
    }

    #[test]
    fn walks_a_straight_line_between_two_squares() {
        let mut labels = Labels::default();
        let right = labels.intern(&vec![1, 0]);
        let board = board(vec![8, 8], &[], &labels);
        let node = |name: Vec<u8>| board.node(&name).unwrap();
        assert_eq!(
            board
                .path(node(vec![0, 0]), node(vec![3, 0]), right.unit)
                .map(|nodes| names(&board, &nodes)),
            Some(vec![vec![1, 0], vec![2, 0], vec![3, 0]])
        );
        assert_eq!(
            board.path(node(vec![0, 0]), node(vec![3, 1]), right.unit),
            None
        );
        assert_eq!(
            board.path(node(vec![2, 2]), node(vec![2, 2]), right.unit),
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
        let mut node = board.node(&vec![0, 0]).unwrap();
        for _ in 0..99 {
            node = board.follow(node, diagonal.unwrap()).unwrap();
        }
        assert_eq!(board.name(node), Some(&vec![99, 99]));
        assert_eq!(board.follow(node, diagonal.unwrap()), None);
    }
}
