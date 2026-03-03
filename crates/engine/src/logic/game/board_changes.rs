use std::collections::HashMap;

use crate::logic::Piece;
use crate::shared::{BoardChange, Position};

/// Applies board changes in order to a mutable pieces map.
///
/// Order is significant: later changes overwrite earlier ones for the same square.
pub(crate) fn apply_to_map(pieces: &mut HashMap<Position, Piece>, changes: &[BoardChange]) {
    for change in changes {
        match &change.piece {
            Some(piece) => {
                pieces.insert(change.position.clone(), piece.clone());
            }
            None => {
                pieces.remove(&change.position);
            }
        }
    }
}

/// Clones `base` and applies `changes` in order, returning the resulting map.
pub(crate) fn simulate_from(
    base: &HashMap<Position, Piece>,
    changes: &[BoardChange],
) -> HashMap<Position, Piece> {
    let mut sim = base.clone();
    apply_to_map(&mut sim, changes);
    sim
}

#[cfg(test)]
mod tests {
    use super::{apply_to_map, simulate_from};
    use crate::logic::Piece;
    use crate::shared::BoardChange;
    use std::collections::HashMap;

    fn piece(code: &str, player: &str) -> Piece {
        Piece::new(code.to_string(), player.to_string())
    }

    #[test]
    fn apply_changes_in_order_set_then_clear_results_empty() {
        let mut pieces = HashMap::new();
        pieces.insert(vec![1, 1], piece("A", "WHITE"));

        let changes = vec![
            BoardChange::set_piece(vec![1, 1], piece("B", "WHITE")),
            BoardChange::clear(&vec![1, 1]),
        ];
        apply_to_map(&mut pieces, &changes);

        assert!(pieces.get(&vec![1, 1]).is_none());
    }

    #[test]
    fn apply_changes_in_order_clear_then_set_results_present() {
        let mut pieces = HashMap::new();
        pieces.insert(vec![2, 2], piece("A", "WHITE"));

        let changes = vec![
            BoardChange::clear(&vec![2, 2]),
            BoardChange::set_piece(vec![2, 2], piece("B", "BLACK")),
        ];
        apply_to_map(&mut pieces, &changes);

        let p = pieces.get(&vec![2, 2]).expect("piece should exist");
        assert_eq!(p.code, "B");
        assert_eq!(p.player, "BLACK");
    }

    #[test]
    fn repeated_idempotent_clears_keep_square_absent() {
        let mut pieces = HashMap::new();
        pieces.insert(vec![3, 3], piece("A", "WHITE"));

        let changes = vec![
            BoardChange::clear(&vec![3, 3]),
            BoardChange::clear(&vec![3, 3]),
        ];
        apply_to_map(&mut pieces, &changes);

        assert!(pieces.get(&vec![3, 3]).is_none());
    }

    #[test]
    fn simulate_from_does_not_mutate_base_map() {
        let mut base = HashMap::new();
        base.insert(vec![4, 4], piece("A", "WHITE"));

        let changes = vec![BoardChange::set_piece(vec![5, 5], piece("B", "BLACK"))];
        let sim = simulate_from(&base, &changes);

        assert!(base.get(&vec![5, 5]).is_none());
        assert!(sim.get(&vec![5, 5]).is_some());
    }
}
