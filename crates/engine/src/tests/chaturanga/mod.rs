#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, Piece};
    use crate::shared::Position;
    use crate::specs::parse_game_spec;
    use crate::tests::support::{go, pick, promotion_options, targets};

    fn load_chaturanga() -> Game {
        parse_game_spec("./specs/chaturanga.json")
            .map(Game::from_spec)
            .expect("Failed to load chaturanga spec")
    }

    fn insert(game: &mut Game, pos: Position, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    // -----------------------------------------------------------------------
    // Spec loads
    // -----------------------------------------------------------------------

    #[test]
    fn test_chaturanga_spec_loads() {
        load_chaturanga(); // panics on error
    }

    // -----------------------------------------------------------------------
    // Gaja (elephant) jumps exactly two squares diagonally, leaping freely
    // -----------------------------------------------------------------------

    #[test]
    fn test_gaja_jumps_diagonally() {
        let mut game = load_chaturanga();
        game.state.pieces.clear();

        insert(&mut game, pos![3, 3], "GAJA", "WHITE");
        insert(&mut game, pos![3, 2], "ASHVA", "WHITE"); // blocker that gaja leaps over
        insert(&mut game, pos![3, 4], "ASHVA", "BLACK"); // keep BLACK alive

        // GAJA at [3,3] can jump to [5,5], [5,1], [1,5], [1,1] — blockers on intermediate
        // squares are irrelevant because the jump is non-sliding.
        pick(&mut game, pos![3, 3]).unwrap();
        let moves = targets(&game);
        assert!(moves.contains(&pos![5, 5]), "gaja should reach [5,5]");
        assert!(moves.contains(&pos![5, 1]), "gaja should reach [5,1]");
        assert!(moves.contains(&pos![1, 5]), "gaja should reach [1,5]");
        assert!(moves.contains(&pos![1, 1]), "gaja should reach [1,1]");
    }

    // -----------------------------------------------------------------------
    // Bare-king win: capturing all non-Raja pieces wins immediately
    // -----------------------------------------------------------------------

    #[test]
    fn test_bare_king_win() {
        let mut game = load_chaturanga();
        game.state.pieces.clear();

        // WHITE has Raja + Ratha; BLACK has only Raja left (already bare).
        // Move WHITE's Ratha to capture BLACK's last piece to reach bare-king condition.
        // Setup: WHITE Raja [4,0], WHITE Ratha [0,5], BLACK Raja [4,7], BLACK Mantri [3,7].
        insert(&mut game, pos![4, 0], "RAJA", "WHITE");
        insert(&mut game, pos![0, 5], "RATHA", "WHITE");
        insert(&mut game, pos![4, 7], "RAJA", "BLACK");
        insert(&mut game, pos![3, 7], "MANTRI", "BLACK");

        // WHITE Ratha slides from [0,5] to [3,5] then we need it to capture [3,7].
        // Simpler: place Ratha at [3,5] and capture Mantri directly.
        game.state.pieces.remove(&pos![0, 5]);
        insert(&mut game, pos![3, 5], "RATHA", "WHITE");

        pick(&mut game, pos![3, 5]).unwrap();
        go(&mut game, pos![3, 7]).unwrap();

        assert!(
            matches!(game.state.phase, GamePhase::GameOver { winner: Some(ref w) } if w == "WHITE"),
            "capturing the last non-Raja BLACK piece should trigger bare-king win for WHITE"
        );
    }

    // -----------------------------------------------------------------------
    // Padati (foot soldier) promotes only to Mantri
    // -----------------------------------------------------------------------

    #[test]
    fn test_padati_promotes_to_mantri() {
        let mut game = load_chaturanga();
        game.state.pieces.clear();

        insert(&mut game, pos![3, 6], "PADATI", "WHITE");
        insert(&mut game, pos![4, 7], "RAJA", "BLACK"); // keep BLACK alive

        // Padati steps forward to rank 7 → transformation required.
        pick(&mut game, pos![3, 6]).unwrap();
        go(&mut game, pos![3, 7]).unwrap();

        // A promotion with one option asks nothing: the padati simply becomes a mantri.
        assert!(promotion_options(&game).is_empty());
        assert_eq!(
            game.state.pieces.get(&pos![3, 7]).map(|p| p.code.as_str()),
            Some("MANTRI"),
            "padati reaching the last rank must become a MANTRI"
        );
    }
}
