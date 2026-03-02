// TODO: The current test pieces (LINE_ATTACKER, DIAGONAL_ATTACKER, JUMP_ATTACKER, etc.) cover
// the main behavioral categories but may not exercise all valid custom movement patterns —
// e.g. L-shaped repeating moves, multi-step jumps, or other exotic specs. A future improvement
// could be to load threat calculation directly from a richer set of spec-defined pieces rather
// than hand-coding representative archetypes here.
#[cfg(test)]
mod tests {
    use crate::logic::{Game, Piece};
    use crate::specs::parse_game_spec;
    use crate::shared::Position;

    fn load_game() -> Game {
        parse_game_spec("./src/tests/attack_map/spec.json")
            .map(Game::from_spec)
            .expect("Failed to load attack_map test spec")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, code: &str, player: &str) {
        game.state.pieces.insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    // -------------------------------------------------------------------------
    // LINE_ATTACKER tests
    // -------------------------------------------------------------------------

    /// LINE_ATTACKER at [5,5] on an empty 10×10 board.
    /// Threatens all 18 squares in the cross (no pieces to block).
    #[test]
    fn test_line_attacker_empty_board() {
        let mut game = load_game();
        insert(&mut game, vec![5, 5], "LINE_ATTACKER", "WHITE");

        let threats = game.attacked_by("WHITE");

        // Compute expected cross squares.
        let mut expected: Vec<Position> = vec![];
        // Right: [6,5]..[9,5]
        for x in 6u8..=9 { expected.push(vec![x, 5]); }
        // Left:  [4,5]..[0,5]
        for x in 0u8..=4 { expected.push(vec![x, 5]); }
        // Up:    [5,6]..[5,9]
        for y in 6u8..=9 { expected.push(vec![5, y]); }
        // Down:  [5,0]..[5,4]
        for y in 0u8..=4 { expected.push(vec![5, y]); }

        assert_eq!(threats.len(), 18, "Expected 18 threatened squares, got {}", threats.len());
        for pos in &expected {
            assert!(threats.contains(pos), "Expected {:?} to be threatened", pos);
        }
        assert!(!threats.contains(&vec![5u8, 5u8]), "[5,5] (source) must not be in the threat set");
    }

    /// LINE_ATTACKER (WHITE) at [5,5], WHITE DUMMY at [5,7].
    /// Ally at [5,7] blocks: [5,6] threatened, [5,7] and beyond not.
    #[test]
    fn test_line_attacker_blocked_by_ally() {
        let mut game = load_game();
        insert(&mut game, vec![5, 5], "LINE_ATTACKER", "WHITE");
        insert(&mut game, vec![5, 7], "DUMMY", "WHITE");

        let threats = game.attacked_by("WHITE");

        assert!(threats.contains(&vec![5u8, 6u8]), "[5,6] should be threatened (empty square before ally)");
        assert!(!threats.contains(&vec![5u8, 7u8]), "[5,7] (ally) must not be threatened");
        assert!(!threats.contains(&vec![5u8, 8u8]), "[5,8] must not be threatened (behind ally)");
    }

    /// LINE_ATTACKER (WHITE) at [5,5], BLACK DUMMY at [5,7].
    /// Enemy at [5,7] is threatened and stops the ray: [5,6] and [5,7] threatened, [5,8] not.
    #[test]
    fn test_line_attacker_blocked_by_enemy() {
        let mut game = load_game();
        insert(&mut game, vec![5, 5], "LINE_ATTACKER", "WHITE");
        insert(&mut game, vec![5, 7], "DUMMY", "BLACK");

        let threats = game.attacked_by("WHITE");

        assert!(threats.contains(&vec![5u8, 6u8]), "[5,6] should be threatened (empty)");
        assert!(threats.contains(&vec![5u8, 7u8]), "[5,7] (enemy) should be threatened");
        assert!(!threats.contains(&vec![5u8, 8u8]), "[5,8] must not be threatened (behind enemy)");
    }

    // -------------------------------------------------------------------------
    // FORWARD_ONLY exclusion test
    // -------------------------------------------------------------------------

    /// FORWARD_ONLY has only EMPTY→MOVE, so it never contributes to the attack map.
    #[test]
    fn test_forward_only_piece_does_not_threaten() {
        let mut game = load_game();
        insert(&mut game, vec![5, 5], "FORWARD_ONLY", "WHITE");

        let threats = game.attacked_by("WHITE");
        assert!(threats.is_empty(), "FORWARD_ONLY must not threaten any square (no ENEMY→CAPTURE)");
    }

    // -------------------------------------------------------------------------
    // JUMP_ATTACKER test
    // -------------------------------------------------------------------------

    /// JUMP_ATTACKER at [5,5] on an empty board threatens exactly the 8 knight squares.
    #[test]
    fn test_jump_attacker() {
        let mut game = load_game();
        insert(&mut game, vec![5, 5], "JUMP_ATTACKER", "WHITE");

        let threats = game.attacked_by("WHITE");

        let expected: Vec<Position> = vec![
            vec![7, 6], vec![7, 4],
            vec![3, 6], vec![3, 4],
            vec![6, 7], vec![6, 3],
            vec![4, 7], vec![4, 3],
        ];

        assert_eq!(threats.len(), 8, "Expected exactly 8 knight squares, got {}", threats.len());
        for pos in &expected {
            assert!(threats.contains(pos), "Expected knight square {:?} to be threatened", pos);
        }
    }
}
