#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load_chaturanga() -> Game {
        parse_game_spec("./specs/chaturanga.json")
            .map(Game::from_spec)
            .expect("Failed to load chaturanga spec")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, code: &str, player: &str) {
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

        insert(&mut game, vec![3, 3], "GAJA",  "WHITE");
        insert(&mut game, vec![3, 2], "ASHVA", "WHITE"); // blocker that gaja leaps over
        insert(&mut game, vec![3, 4], "ASHVA", "BLACK"); // keep BLACK alive

        // GAJA at [3,3] can jump to [5,5], [5,1], [1,5], [1,1] — blockers on intermediate
        // squares are irrelevant because the jump is non-sliding.
        game.transition(GameTransition::CalculateMoves { position: vec![3, 3] }).unwrap();
        let moves = game.state.available_moves.as_ref().unwrap();
        assert!(moves.contains_key(&vec![5, 5]), "gaja should reach [5,5]");
        assert!(moves.contains_key(&vec![5, 1]), "gaja should reach [5,1]");
        assert!(moves.contains_key(&vec![1, 5]), "gaja should reach [1,5]");
        assert!(moves.contains_key(&vec![1, 1]), "gaja should reach [1,1]");
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
        insert(&mut game, vec![4, 0], "RAJA",  "WHITE");
        insert(&mut game, vec![0, 5], "RATHA", "WHITE");
        insert(&mut game, vec![4, 7], "RAJA",  "BLACK");
        insert(&mut game, vec![3, 7], "MANTRI","BLACK");

        // WHITE Ratha slides from [0,5] to [3,5] then we need it to capture [3,7].
        // Simpler: place Ratha at [3,5] and capture Mantri directly.
        game.state.pieces.remove(&vec![0, 5]);
        insert(&mut game, vec![3, 5], "RATHA", "WHITE");

        game.transition(GameTransition::CalculateMoves { position: vec![3, 5] }).unwrap();
        game.transition(GameTransition::ExecuteMove  { position: vec![3, 7] }).unwrap();

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

        insert(&mut game, vec![3, 6], "PADATI", "WHITE");
        insert(&mut game, vec![4, 7], "RAJA",   "BLACK"); // keep BLACK alive

        // Padati steps forward to rank 7 → transformation required.
        game.transition(GameTransition::CalculateMoves { position: vec![3, 6] }).unwrap();
        game.transition(GameTransition::ExecuteMove  { position: vec![3, 7] }).unwrap();

        // Engine should be in Transforming phase with only MANTRI as option.
        assert!(
            matches!(game.state.phase, GamePhase::Transforming { ref options, .. } if options == &vec!["MANTRI".to_string()]),
            "padati reaching the last rank must offer only MANTRI as promotion"
        );
    }
}
