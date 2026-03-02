#[cfg(test)]
mod tests {
    use crate::logic::{Game, GameTransition};
    use crate::specs::parse_game_spec;

    fn load_chess() -> Game {
        parse_game_spec("./specs/chess.json")
            .map(Game::from_spec)
            .expect("Failed to load chess spec")
    }

    /// Clear all pieces between two x positions (exclusive) on a given rank.
    /// Used to open the castling path.
    fn clear_rank_range(game: &mut Game, x_start: u8, x_end: u8, rank: u8) {
        for x in x_start..=x_end {
            game.state.pieces.remove(&vec![x, rank]);
        }
    }

    // -------------------------------------------------------------------------
    // WHITE kingside castling (move id 9, step [2,0])
    // King: [4,0] → [6,0], Rook: [7,0] → [5,0]
    // -------------------------------------------------------------------------

    #[test]
    fn test_white_can_castle_kingside() {
        let mut game = load_chess();
        // Remove knight [6,0] and bishop [5,0] to open the kingside path.
        clear_rank_range(&mut game, 5, 6, 0);

        game.transition(GameTransition::CalculateMoves { position: vec![4, 0] }).unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();
        assert!(
            moves.contains_key(&vec![6u8, 0u8]),
            "WHITE should be able to castle kingside to [6,0]"
        );
    }

    #[test]
    fn test_white_kingside_castle_repositions_rook() {
        let mut game = load_chess();
        clear_rank_range(&mut game, 5, 6, 0);

        game.transition(GameTransition::CalculateMoves { position: vec![4, 0] }).unwrap();
        game.transition(GameTransition::ExecuteMove { position: vec![6, 0] }).unwrap();

        assert!(
            game.state.pieces.get(&vec![6u8, 0u8]).is_some(),
            "King should be at [6,0] after kingside castle"
        );
        assert_eq!(
            game.state.pieces.get(&vec![6u8, 0u8]).unwrap().code,
            "KING",
            "Piece at [6,0] should be the KING"
        );
        assert!(
            game.state.pieces.get(&vec![5u8, 0u8]).is_some(),
            "Rook should be at [5,0] after kingside castle"
        );
        assert_eq!(
            game.state.pieces.get(&vec![5u8, 0u8]).unwrap().code,
            "ROOK",
            "Piece at [5,0] should be the ROOK"
        );
        assert!(
            game.state.pieces.get(&vec![4u8, 0u8]).is_none(),
            "King's original square [4,0] should be cleared"
        );
        assert!(
            game.state.pieces.get(&vec![7u8, 0u8]).is_none(),
            "Rook's original square [7,0] should be cleared"
        );
    }

    // -------------------------------------------------------------------------
    // WHITE queenside castling (move id 8, step [-2,0])
    // King: [4,0] → [2,0], Rook: [0,0] → [3,0]
    // -------------------------------------------------------------------------

    #[test]
    fn test_white_can_castle_queenside() {
        let mut game = load_chess();
        // Remove queen [3,0], bishop [2,0], and knight [1,0] to open queenside path.
        clear_rank_range(&mut game, 1, 3, 0);

        game.transition(GameTransition::CalculateMoves { position: vec![4, 0] }).unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();
        assert!(
            moves.contains_key(&vec![2u8, 0u8]),
            "WHITE should be able to castle queenside to [2,0]"
        );
    }

    #[test]
    fn test_white_queenside_castle_repositions_rook() {
        let mut game = load_chess();
        clear_rank_range(&mut game, 1, 3, 0);

        game.transition(GameTransition::CalculateMoves { position: vec![4, 0] }).unwrap();
        game.transition(GameTransition::ExecuteMove { position: vec![2, 0] }).unwrap();

        assert_eq!(
            game.state.pieces.get(&vec![2u8, 0u8]).unwrap().code, "KING",
            "KING should be at [2,0]"
        );
        assert_eq!(
            game.state.pieces.get(&vec![3u8, 0u8]).unwrap().code, "ROOK",
            "ROOK should be at [3,0]"
        );
        assert!(game.state.pieces.get(&vec![4u8, 0u8]).is_none(), "Source [4,0] cleared");
        assert!(game.state.pieces.get(&vec![0u8, 0u8]).is_none(), "Rook source [0,0] cleared");
    }

    // -------------------------------------------------------------------------
    // Castling blocked conditions
    // -------------------------------------------------------------------------

    #[test]
    fn test_castling_blocked_when_king_has_moved() {
        let mut game = load_chess();
        clear_rank_range(&mut game, 5, 6, 0);

        // Make the king move and come back (increment total_moves).
        game.state.pieces.get_mut(&vec![4u8, 0u8]).unwrap().total_moves = 1;

        game.transition(GameTransition::CalculateMoves { position: vec![4, 0] }).unwrap();
        let moves = game.state.available_moves.as_ref();
        assert!(
            moves.map_or(true, |m| !m.contains_key(&vec![6u8, 0u8])),
            "Castling should be blocked after king has moved"
        );
    }

    #[test]
    fn test_castling_blocked_when_rook_has_moved() {
        let mut game = load_chess();
        clear_rank_range(&mut game, 5, 6, 0);

        // Mark the h1 rook as having moved.
        game.state.pieces.get_mut(&vec![7u8, 0u8]).unwrap().total_moves = 1;

        game.transition(GameTransition::CalculateMoves { position: vec![4, 0] }).unwrap();
        let moves = game.state.available_moves.as_ref();
        assert!(
            moves.map_or(true, |m| !m.contains_key(&vec![6u8, 0u8])),
            "Kingside castling should be blocked after h1 rook has moved"
        );
    }

    #[test]
    fn test_castling_blocked_when_path_not_empty() {
        let mut game = load_chess();
        // Only remove the knight at [6,0]; leave bishop at [5,0] to block f1.
        game.state.pieces.remove(&vec![6u8, 0u8]);

        game.transition(GameTransition::CalculateMoves { position: vec![4, 0] }).unwrap();
        let moves = game.state.available_moves.as_ref();
        assert!(
            moves.map_or(true, |m| !m.contains_key(&vec![6u8, 0u8])),
            "Kingside castling should be blocked when bishop is on f1=[5,0]"
        );
    }

    // -------------------------------------------------------------------------
    // BLACK castling
    // BLACK move id 8 (spec step [-2,0] → actual [2,0]): king [4,7]→[6,7], rook [7,7]→[5,7]
    // BLACK move id 9 (spec step [2,0] → actual [-2,0]):  king [4,7]→[2,7], rook [0,7]→[3,7]
    // -------------------------------------------------------------------------

    #[test]
    fn test_black_can_castle_kingside() {
        let mut game = load_chess();
        // Clear WHITE pieces to avoid blocking the king (WHITE goes first, skip).
        // Remove BLACK's knight [6,7] and bishop [5,7] to open the kingside path.
        clear_rank_range(&mut game, 5, 6, 7);

        // Advance to BLACK's turn by making a dummy WHITE move first.
        game.transition(GameTransition::CalculateMoves { position: vec![0, 1] }).unwrap();
        game.transition(GameTransition::ExecuteMove { position: vec![0, 2] }).unwrap();

        // Now it's BLACK's turn.
        game.transition(GameTransition::CalculateMoves { position: vec![4, 7] }).unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();
        assert!(
            moves.contains_key(&vec![6u8, 7u8]),
            "BLACK should be able to castle kingside to [6,7]"
        );
    }

    #[test]
    fn test_black_can_castle_queenside() {
        let mut game = load_chess();
        // Remove BLACK's queen [3,7], bishop [2,7], knight [1,7] to open queenside.
        clear_rank_range(&mut game, 1, 3, 7);

        // Advance to BLACK's turn.
        game.transition(GameTransition::CalculateMoves { position: vec![0, 1] }).unwrap();
        game.transition(GameTransition::ExecuteMove { position: vec![0, 2] }).unwrap();

        game.transition(GameTransition::CalculateMoves { position: vec![4, 7] }).unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();
        assert!(
            moves.contains_key(&vec![2u8, 7u8]),
            "BLACK should be able to castle queenside to [2,7]"
        );
    }
}
