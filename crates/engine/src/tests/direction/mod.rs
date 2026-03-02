#[cfg(test)]
mod tests {
    use crate::logic::{Game, GameTransition};
    use crate::specs::{parse_game_spec, GameSpecError};
    use crate::shared::apply_direction;

    // ---------------------------------------------------------------------------
    // apply_direction unit tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_identity_matrix_is_unchanged() {
        let identity = [[1, 0], [0, 1]];
        assert_eq!(apply_direction(&identity, &vec![0, 1]), vec![0, 1]);
        assert_eq!(apply_direction(&identity, &vec![1, 0]), vec![1, 0]);
        assert_eq!(apply_direction(&identity, &vec![1, 1]), vec![1, 1]);
    }

    #[test]
    fn test_180_rotation_reverses_both_axes() {
        let rotate_180 = [[-1, 0], [0, -1]];
        assert_eq!(apply_direction(&rotate_180, &vec![0, 1]),  vec![0, -1]);
        assert_eq!(apply_direction(&rotate_180, &vec![1, 0]),  vec![-1, 0]);
        assert_eq!(apply_direction(&rotate_180, &vec![1, 1]),  vec![-1, -1]);
        assert_eq!(apply_direction(&rotate_180, &vec![-1, 1]), vec![1, -1]);
    }

    #[test]
    fn test_90_clockwise_rotation() {
        // SILVER: canonical [0, 1] (forward/up) becomes [1, 0] (right)
        let rotate_90cw = [[0, 1], [-1, 0]];
        assert_eq!(apply_direction(&rotate_90cw, &vec![0,  1]),  vec![ 1,  0]); // up    → right
        assert_eq!(apply_direction(&rotate_90cw, &vec![0, -1]),  vec![-1,  0]); // down  → left
        assert_eq!(apply_direction(&rotate_90cw, &vec![1,  0]),  vec![ 0, -1]); // right → down
        assert_eq!(apply_direction(&rotate_90cw, &vec![-1, 0]),  vec![ 0,  1]); // left  → up
    }

    #[test]
    fn test_90_counter_clockwise_rotation() {
        // GOLD: canonical [0, 1] (forward/up) becomes [-1, 0] (left)
        let rotate_90ccw = [[0, -1], [1, 0]];
        assert_eq!(apply_direction(&rotate_90ccw, &vec![0,  1]),  vec![-1,  0]); // up    → left
        assert_eq!(apply_direction(&rotate_90ccw, &vec![0, -1]),  vec![ 1,  0]); // down  → right
        assert_eq!(apply_direction(&rotate_90ccw, &vec![1,  0]),  vec![ 0,  1]); // right → up
        assert_eq!(apply_direction(&rotate_90ccw, &vec![-1, 0]),  vec![ 0, -1]); // left  → down
    }

    // ---------------------------------------------------------------------------
    // 4-player spec integration test
    // ---------------------------------------------------------------------------

    #[test]
    fn test_4player_spec_loads_and_all_queens_can_move() -> Result<(), GameSpecError> {
        let game_spec = parse_game_spec("./specs/4player_chess.json")?;
        let mut game = Game::from_spec(game_spec);

        // WHITE queen is at [3, 0] — should be able to move upward along the file.
        assert!(game.transition(GameTransition::CalculateMoves { position: vec![3, 0] }).is_ok());
        let white_moves = game.state.available_moves.as_ref().unwrap();
        assert!(white_moves.contains_key(&vec![3, 1]), "WHITE queen should reach [3, 1]");
        assert!(white_moves.contains_key(&vec![3, 2]), "WHITE queen should reach [3, 2]");

        // Advance turn to BLACK (skip WHITE move — use a neutral square that has no piece).
        // Instead, just directly verify each player's queen moves by resetting turns.
        // We do this by executing a move for WHITE first.
        assert!(game.transition(GameTransition::ExecuteMove { position: vec![3, 1] }).is_ok());

        // BLACK queen is at [4, 7] — should be able to move downward (toward lower y).
        assert!(game.transition(GameTransition::CalculateMoves { position: vec![4, 7] }).is_ok());
        let black_moves = game.state.available_moves.as_ref().unwrap();
        assert!(black_moves.contains_key(&vec![4, 6]), "BLACK queen should reach [4, 6]");
        assert!(black_moves.contains_key(&vec![4, 5]), "BLACK queen should reach [4, 5]");
        assert!(game.transition(GameTransition::ExecuteMove { position: vec![4, 6] }).is_ok());

        // SILVER queen is at [0, 4] — should be able to move rightward (toward higher x).
        assert!(game.transition(GameTransition::CalculateMoves { position: vec![0, 4] }).is_ok());
        let silver_moves = game.state.available_moves.as_ref().unwrap();
        assert!(silver_moves.contains_key(&vec![1, 4]), "SILVER queen should reach [1, 4]");
        assert!(silver_moves.contains_key(&vec![2, 4]), "SILVER queen should reach [2, 4]");
        assert!(game.transition(GameTransition::ExecuteMove { position: vec![1, 4] }).is_ok());

        // GOLD queen is at [7, 3] — should be able to move leftward (toward lower x).
        assert!(game.transition(GameTransition::CalculateMoves { position: vec![7, 3] }).is_ok());
        let gold_moves = game.state.available_moves.as_ref().unwrap();
        assert!(gold_moves.contains_key(&vec![6, 3]), "GOLD queen should reach [6, 3]");
        assert!(gold_moves.contains_key(&vec![5, 3]), "GOLD queen should reach [5, 3]");

        Ok(())
    }
}
