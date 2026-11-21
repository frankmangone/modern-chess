#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::logic::Board;
    use crate::shared::Position;
    use crate::specs::BoardSpec;

    fn board(dimensions: Vec<u8>, disabled: &[Position]) -> Board {
        Board::from_spec(BoardSpec {
            dimensions,
            disabled_positions: disabled.iter().cloned().collect::<HashSet<_>>(),
        })
    }

    #[test]
    fn lists_every_position_of_a_3d_board() {
        let positions = board(vec![4, 4, 4], &[]).all_positions();
        assert_eq!(positions.len(), 64);
        assert_eq!(positions.first(), Some(&vec![0, 0, 0]));
        assert_eq!(positions.last(), Some(&vec![3, 3, 3]));
        let mut sorted = positions.clone();
        sorted.sort();
        assert_eq!(positions, sorted);
    }

    #[test]
    fn lists_2d_positions_in_the_same_order_as_before() {
        assert_eq!(
            board(vec![2, 3], &[vec![1, 1]]).all_positions(),
            vec![vec![0, 0], vec![0, 1], vec![0, 2], vec![1, 0], vec![1, 2]]
        );
    }

    #[test]
    fn refuses_positions_of_the_wrong_length() {
        let chess = board(vec![8, 8], &[]);
        assert!(chess.is_position_valid(&vec![3, 3]));
        assert!(!chess.is_position_valid(&vec![3]));
        assert!(!chess.is_position_valid(&vec![3, 3, 0]));
    }

    #[test]
    fn neighbourhoods_reach_along_every_axis() {
        let cube = board(vec![4, 4, 4], &[]);
        assert_eq!(cube.positions_within(&vec![1, 1, 1], 1).len(), 27);
        assert_eq!(cube.positions_within(&vec![0, 0, 0], 1).len(), 8);
        assert_eq!(cube.positions_within(&vec![0, 0, 0], 2).len(), 27);
        assert!(cube.positions_within(&vec![1, 1], 1).is_empty());
        assert_eq!(
            board(vec![8, 8], &[vec![1, 1]])
                .positions_within(&vec![0, 0], 1)
                .len(),
            3
        );
    }
}
