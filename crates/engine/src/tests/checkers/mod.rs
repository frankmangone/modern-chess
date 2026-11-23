#[cfg(test)]
mod tests {
    use crate::logic::{Game, Piece};
    use crate::shared::Position;
    use crate::specs::parse_game_spec;
    use crate::tests::support::{
        go, moves_from, phase_is_choosing, pick, promotion_options, remaining,
    };

    fn load() -> Game {
        parse_game_spec("./specs/checkers.json")
            .map(Game::from_spec)
            .expect("Failed to load checkers spec")
    }

    fn insert(game: &mut Game, pos: Position, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    fn moves_for(game: &mut Game, pos: Position) -> Vec<Position> {
        moves_from(game, pos)
    }

    // -----------------------------------------------------------------------
    // Spec loads
    // -----------------------------------------------------------------------

    #[test]
    fn test_checkers_spec_loads() {
        load();
    }

    // -----------------------------------------------------------------------
    // Board is 8×8
    // -----------------------------------------------------------------------

    #[test]
    fn test_checkers_board_dimensions() {
        let game = load();
        assert_eq!(game.board.dimensions, vec![8u8, 8u8]);
    }

    // -----------------------------------------------------------------------
    // MAN moves forward diagonally only (no backward moves)
    // -----------------------------------------------------------------------

    #[test]
    fn test_man_moves_forward_only() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![4, 4], "MAN", "RED");

        let dests = moves_for(&mut game, pos![4, 4]);

        // forward-right and forward-left for RED
        assert!(dests.contains(&pos![5, 5]), "man must reach [5,5]");
        assert!(dests.contains(&pos![3, 5]), "man must reach [3,5]");

        // no backward moves
        assert!(
            !dests.contains(&pos![5, 3]),
            "man must NOT move backward to [5,3]"
        );
        assert!(
            !dests.contains(&pos![3, 3]),
            "man must NOT move backward to [3,3]"
        );
    }

    // -----------------------------------------------------------------------
    // MAN captures diagonally forward (jump over enemy, land on empty)
    // -----------------------------------------------------------------------

    #[test]
    fn test_man_captures_by_jumping() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![3, 3], "MAN", "RED");
        insert(&mut game, pos![4, 4], "MAN", "BLACK"); // enemy to jump over

        let dests = moves_for(&mut game, pos![3, 3]);

        // jump over [4,4] and land on [5,5]
        assert!(
            dests.contains(&pos![5, 5]),
            "man must jump over enemy to [5,5]"
        );
    }

    #[test]
    fn test_man_cannot_jump_over_ally() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![3, 3], "MAN", "RED");
        insert(&mut game, pos![4, 4], "MAN", "RED"); // ally — cannot jump

        let dests = moves_for(&mut game, pos![3, 3]);

        assert!(!dests.contains(&pos![5, 5]), "man must NOT jump over ally");
    }

    #[test]
    fn test_man_cannot_jump_to_occupied_landing() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![3, 3], "MAN", "RED");
        insert(&mut game, pos![4, 4], "MAN", "BLACK"); // enemy to jump over
        insert(&mut game, pos![5, 5], "MAN", "RED"); // occupied landing square

        let dests = moves_for(&mut game, pos![3, 3]);

        assert!(
            !dests.contains(&pos![5, 5]),
            "man must NOT land on occupied square"
        );
    }

    // -----------------------------------------------------------------------
    // Multi-jump: Jumping phase entered after a capture with continuations
    // -----------------------------------------------------------------------

    #[test]
    fn test_multi_jump_enters_jumping_phase() {
        let mut game = load();
        game.state.pieces.clear();

        // RED man at [3,1], two BLACK men forming a double-jump path
        insert(&mut game, pos![3, 1], "MAN", "RED");
        insert(&mut game, pos![4, 2], "MAN", "BLACK"); // first enemy
        insert(&mut game, pos![4, 4], "MAN", "BLACK"); // second enemy (after first jump lands at [5,3])

        // calculate moves for RED man
        pick(&mut game, pos![3, 1]).unwrap();

        // execute the first jump: land at [5,3]
        go(&mut game, pos![5, 3]).unwrap();

        // After landing at [5,3], enemy at [4,4] can be jumped to [3,5].
        // Engine must enter a locked Moving phase (jump continuation).
        assert!(
            game.state.continuing.is_some()
                && !remaining(&game).is_empty()
                && remaining(&game).iter().all(|way| way.captures),
            "the jump must go on, offering only captures, when further captures exist after a jump"
        );

        // The locked position must be the landing square [5,3].
        assert_eq!(
            game.state.continuing,
            Some(pos![5u8, 3u8]),
            "locked position must be [5,3]"
        );
        assert_eq!(game.choice().start, Some(pos![5u8, 3u8]));
    }

    #[test]
    fn test_multi_jump_completes_on_second_capture() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![3, 1], "MAN", "RED");
        insert(&mut game, pos![4, 2], "MAN", "BLACK"); // first enemy (will be captured)
        insert(&mut game, pos![4, 4], "MAN", "BLACK"); // second enemy (will be captured)
        insert(&mut game, pos![0, 7], "MAN", "BLACK"); // surviving BLACK piece so game continues

        // First jump: [3,1] over [4,2] → [5,3]
        pick(&mut game, pos![3, 1]).unwrap();
        go(&mut game, pos![5, 3]).unwrap();

        // Should be in locked Moving phase (jump continuation)
        assert_eq!(game.state.continuing, Some(pos![5, 3]));

        // Second jump: [5,3] over [4,4] → [3,5]
        go(&mut game, pos![3, 5]).unwrap();

        // No more jumps → turn advances → the next player is choosing
        assert!(
            phase_is_choosing(&game),
            "game must be choosing a new turn after the last jump"
        );

        // Both BLACK men should be captured
        assert!(
            game.state.pieces.get(&pos![4, 2]).is_none(),
            "first captured man must be removed"
        );
        assert!(
            game.state.pieces.get(&pos![4, 4]).is_none(),
            "second captured man must be removed"
        );

        // RED man ends at [3,5]
        assert_eq!(
            game.state.pieces.get(&pos![3, 5]).map(|p| p.code.as_str()),
            Some("MAN"),
            "RED man must land at [3,5]"
        );

        // Turn passed to BLACK
        assert_eq!(game.current_player(), "BLACK");
    }

    // -----------------------------------------------------------------------
    // Mandatory capture: non-capture moves hidden when capture is available
    // -----------------------------------------------------------------------

    #[test]
    fn test_mandatory_capture_hides_regular_moves() {
        let mut game = load();
        game.state.pieces.clear();

        // RED man has both a regular move and a capture available
        insert(&mut game, pos![3, 3], "MAN", "RED");
        insert(&mut game, pos![4, 4], "MAN", "BLACK"); // enemy → jump to [5,5]
                                                       // [2,4] is empty (regular move available too, but must be hidden)

        let dests = moves_for(&mut game, pos![3, 3]);

        // Regular move to [2,4] must be hidden
        assert!(
            !dests.contains(&pos![2, 4]),
            "regular move must be hidden when capture is available"
        );
        // Jump to [5,5] must be shown
        assert!(dests.contains(&pos![5, 5]), "capture jump must be shown");
    }

    // -----------------------------------------------------------------------
    // KING moves in all 4 diagonal directions
    // -----------------------------------------------------------------------

    #[test]
    fn test_king_moves_in_all_directions() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![4, 4], "KING", "RED");

        let dests = moves_for(&mut game, pos![4, 4]);

        assert!(dests.contains(&pos![5, 5]), "king must reach [5,5]");
        assert!(dests.contains(&pos![3, 5]), "king must reach [3,5]");
        assert!(dests.contains(&pos![5, 3]), "king must reach [5,3]");
        assert!(dests.contains(&pos![3, 3]), "king must reach [3,3]");
    }

    #[test]
    fn test_king_captures_backward() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![4, 4], "KING", "RED");
        insert(&mut game, pos![3, 3], "MAN", "BLACK"); // enemy behind

        let dests = moves_for(&mut game, pos![4, 4]);

        // King can jump backward over enemy at [3,3] to land at [2,2]
        assert!(
            dests.contains(&pos![2, 2]),
            "king must capture backward to [2,2]"
        );
    }

    // -----------------------------------------------------------------------
    // MAN promotes to KING at opponent's back rank
    // -----------------------------------------------------------------------

    #[test]
    fn test_man_promotes_to_king_at_back_rank() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![3, 6], "MAN", "RED");

        pick(&mut game, pos![3, 6]).unwrap();
        go(&mut game, pos![4, 7]).unwrap();

        assert!(
            promotion_options(&game).is_empty(),
            "a promotion with one option asks nothing"
        );
        assert_eq!(
            game.state.pieces.get(&pos![4, 7]).map(|p| p.code.as_str()),
            Some("KING"),
            "the man becomes a king on reaching the back rank"
        );
    }

    // -----------------------------------------------------------------------
    // Crowning: the standard rule (a jump that reaches the far rank is a capture like
    // any other, and the turn ends there)
    // -----------------------------------------------------------------------

    #[test]
    fn a_crowning_jump_is_forced_like_any_capture() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![3, 5], "MAN", "RED"); // can jump [4,6] and be crowned on [5,7]
        insert(&mut game, pos![4, 6], "MAN", "BLACK");
        insert(&mut game, pos![0, 2], "MAN", "RED"); // has a plain step only

        game.work_out_ways();
        let ways = remaining(&game);
        assert!(
            !ways.is_empty() && ways.iter().all(|way| way.captures),
            "only captures may be offered, and the crowning jump is one"
        );
        assert_eq!(moves_for(&mut game, pos![3, 5]), vec![pos![5, 7]]);
        assert!(
            moves_for(&mut game, pos![0, 2]).is_empty(),
            "a plain step is hidden while a capture exists"
        );
    }

    #[test]
    fn a_multi_jump_may_end_with_a_crowning_jump() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![3, 3], "MAN", "RED");
        insert(&mut game, pos![4, 4], "MAN", "BLACK"); // first jump: [3,3] to [5,5]
        insert(&mut game, pos![4, 6], "MAN", "BLACK"); // second: [5,5] to [3,7], crowned
        insert(&mut game, pos![0, 7], "MAN", "BLACK"); // keeps BLACK in the game

        pick(&mut game, pos![3, 3]).unwrap();
        go(&mut game, pos![5, 5]).unwrap();
        assert_eq!(game.state.continuing, Some(pos![5u8, 5u8]));
        go(&mut game, pos![3, 7]).unwrap();

        assert_eq!(
            game.state.pieces.get(&pos![3, 7]).map(|p| p.code.as_str()),
            Some("KING"),
            "the man is crowned by the jump that reaches the far rank"
        );
        assert!(game.state.pieces.get(&pos![4, 6]).is_none());
        assert_eq!(game.current_player(), "BLACK");
    }

    #[test]
    fn crowning_ends_the_turn_even_when_the_new_king_could_jump() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, pos![3, 5], "MAN", "RED");
        insert(&mut game, pos![4, 6], "MAN", "BLACK"); // jumped: [3,5] to [5,7]
        insert(&mut game, pos![6, 6], "MAN", "BLACK"); // the new king could jump it backwards

        pick(&mut game, pos![3, 5]).unwrap();
        go(&mut game, pos![5, 7]).unwrap();

        assert_eq!(
            game.state.pieces.get(&pos![5, 7]).map(|p| p.code.as_str()),
            Some("KING")
        );
        assert!(
            game.state.continuing.is_none(),
            "no continuation after crowning"
        );
        assert_eq!(game.current_player(), "BLACK", "the turn passes");
    }
}
