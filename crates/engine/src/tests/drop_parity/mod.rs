#[cfg(test)]
mod tests {
    use crate::logic::{Game, Piece};
    use crate::specs::parse_game_spec;

    fn load_shogi() -> Game {
        parse_game_spec("./specs/shogi.json")
            .map(Game::from_spec)
            .expect("Failed to load shogi spec")
    }

    #[test]
    fn test_any_legal_moves_matches_drop_generation_when_drop_exists() {
        let mut game = load_shogi();
        game.state.pieces.clear();
        game.leader.clear(); // isolate drop pipeline parity from leader-check constraints

        let current_player = game.current_player();
        game.state
            .hand
            .entry(current_player.clone())
            .or_default()
            .insert("PAWN".to_string(), 1);

        let drops = game.compute_drop_squares("PAWN", &current_player);
        assert!(
            !drops.is_empty(),
            "Expected at least one legal drop square for PAWN"
        );
        assert!(
            game.any_legal_moves(),
            "any_legal_moves should be true when legal drop squares exist"
        );
    }

    #[test]
    fn test_any_legal_moves_matches_drop_generation_when_no_drop_exists() {
        let mut game = load_shogi();
        game.state.pieces.clear();
        game.leader.clear(); // isolate drop pipeline parity from leader-check constraints

        let current_player = game.current_player();
        let opponent = game
            .players
            .iter()
            .find(|player| player.as_str() != current_player.as_str())
            .expect("shogi should have two players")
            .clone();

        game.state
            .hand
            .entry(current_player.clone())
            .or_default()
            .insert("PAWN".to_string(), 1);

        // Occupy every square with opponent pieces so no drop square remains.
        for position in game.board.all_positions() {
            game.state
                .pieces
                .insert(position, Piece::new("PAWN".to_string(), opponent.clone()));
        }

        let drops = game.compute_drop_squares("PAWN", &current_player);
        assert!(
            drops.is_empty(),
            "Expected no legal drop squares when board is fully occupied"
        );
        assert!(
            !game.any_legal_moves(),
            "any_legal_moves should be false when no board moves and no drops are legal"
        );
    }
}
