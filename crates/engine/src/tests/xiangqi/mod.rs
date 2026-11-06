#[cfg(test)]
mod tests {
    use crate::logic::{Game, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load() -> Game {
        parse_game_spec("./specs/xiangqi.json")
            .map(Game::from_spec)
            .expect("Failed to load xiangqi spec")
    }

    fn insert(game: &mut Game, pos: Vec<u8>, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    fn moves_for(game: &mut Game, pos: Vec<u8>) -> Vec<Vec<u8>> {
        game.transition(GameTransition::CalculateMoves { position: pos })
            .unwrap();
        let mut dests: Vec<Vec<u8>> = game
            .state
            .available_moves
            .as_ref()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        dests.sort();
        dests
    }

    // -----------------------------------------------------------------------
    // Spec loads
    // -----------------------------------------------------------------------

    #[test]
    fn test_xiangqi_spec_loads() {
        let game = load();
        assert_eq!(game.board.dimensions, vec![9u8, 10u8]);
        assert_eq!(game.name, "XIANGQI");
    }

    // -----------------------------------------------------------------------
    // GENERAL is confined to the palace by zone restriction
    // -----------------------------------------------------------------------

    #[test]
    fn test_general_confined_to_palace() {
        let mut game = load();
        game.state.pieces.clear();

        // RED GENERAL in the middle of the palace, BLACK GENERAL somewhere safe.
        insert(&mut game, vec![4, 1], "GENERAL", "RED");
        insert(&mut game, vec![4, 9], "GENERAL", "BLACK");
        // Shield the file so the generals do not face each other.
        insert(&mut game, vec![4, 5], "SOLDIER", "BLACK");

        let dests = moves_for(&mut game, vec![4, 1]);

        // All reachable squares must be inside RED's palace (cols 3–5, rows 0–2).
        for dest in &dests {
            assert!(
                dest[0] >= 3 && dest[0] <= 5 && dest[1] <= 2,
                "GENERAL must stay in palace, but would reach {:?}",
                dest
            );
        }

        // Specifically: [4,0], [3,1], [5,1], [4,2] are valid palace moves.
        assert!(dests.contains(&vec![4u8, 0u8]), "GENERAL must reach [4,0]");
        assert!(dests.contains(&vec![3u8, 1u8]), "GENERAL must reach [3,1]");
        assert!(dests.contains(&vec![5u8, 1u8]), "GENERAL must reach [5,1]");
        assert!(dests.contains(&vec![4u8, 2u8]), "GENERAL must reach [4,2]");

        // [4,3] is outside the palace — must be absent.
        assert!(
            !dests.contains(&vec![4u8, 3u8]),
            "GENERAL must NOT leave the palace to [4,3]"
        );
    }

    // -----------------------------------------------------------------------
    // ADVISOR is confined to the palace
    // -----------------------------------------------------------------------

    #[test]
    fn test_advisor_confined_to_palace() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![4, 1], "ADVISOR", "RED");
        insert(&mut game, vec![4, 9], "GENERAL", "BLACK");
        // Shield the file so the generals do not face each other.
        insert(&mut game, vec![4, 5], "SOLDIER", "BLACK");
        insert(&mut game, vec![4, 0], "GENERAL", "RED");

        let dests = moves_for(&mut game, vec![4, 1]);

        // Diagonal moves from palace centre: [3,0],[5,0],[3,2],[5,2].
        assert!(dests.contains(&vec![3u8, 0u8]), "ADVISOR must reach [3,0]");
        assert!(dests.contains(&vec![5u8, 0u8]), "ADVISOR must reach [5,0]");
        assert!(dests.contains(&vec![3u8, 2u8]), "ADVISOR must reach [3,2]");
        assert!(dests.contains(&vec![5u8, 2u8]), "ADVISOR must reach [5,2]");

        // [6,2] and [2,0] are outside the palace.
        assert!(
            !dests.contains(&vec![6u8, 2u8]),
            "ADVISOR must NOT leave the palace"
        );
    }

    // -----------------------------------------------------------------------
    // ELEPHANT cannot cross the river (zone restriction: OWN_SIDE)
    // -----------------------------------------------------------------------

    #[test]
    fn test_elephant_cannot_cross_river() {
        let mut game = load();
        game.state.pieces.clear();

        // RED ELEPHANT at [2,2] — one diagonal step [2,2] away lands on [4,4] (own side, ok).
        // Step [-2,2] lands on [0,4] (own side, ok).
        // Step [2,-2] lands on [4,0] (own side, ok).
        // Step [-2,-2] is off-board.
        // Place at [4,2] so crossing-river moves are possible to test.
        insert(&mut game, vec![4, 2], "ELEPHANT", "RED");
        insert(&mut game, vec![4, 9], "GENERAL", "BLACK");
        // Shield the file so the generals do not face each other.
        insert(&mut game, vec![4, 5], "SOLDIER", "BLACK");
        insert(&mut game, vec![4, 0], "GENERAL", "RED");

        let dests = moves_for(&mut game, vec![4, 2]);

        // Any destination in rows 5–9 must not appear (enemy side / across river).
        for dest in &dests {
            assert!(
                dest[1] <= 4,
                "ELEPHANT must not cross the river, but would reach {:?}",
                dest
            );
        }
    }

    // -----------------------------------------------------------------------
    // ELEPHANT is blocked when its diagonal midpoint is occupied (elephant foot)
    // -----------------------------------------------------------------------

    #[test]
    fn test_elephant_blocked_by_foot() {
        let mut game = load();
        game.state.pieces.clear();

        // RED ELEPHANT at [2,0]; step [2,2] → [4,2].
        // The midpoint (foot) is [3,1]. Block it.
        insert(&mut game, vec![2, 0], "ELEPHANT", "RED");
        insert(&mut game, vec![3, 1], "SOLDIER", "RED"); // blocks the foot
        insert(&mut game, vec![4, 9], "GENERAL", "BLACK");
        // Shield the file so the generals do not face each other.
        insert(&mut game, vec![4, 5], "SOLDIER", "BLACK");
        insert(&mut game, vec![4, 0], "GENERAL", "RED");

        let dests = moves_for(&mut game, vec![2, 0]);

        // [4,2] (step [2,2]) is blocked because midpoint [3,1] is occupied.
        assert!(
            !dests.contains(&vec![4u8, 2u8]),
            "ELEPHANT move must be blocked when its foot [3,1] is occupied"
        );
    }

    // -----------------------------------------------------------------------
    // HORSE is blocked when its leg square is occupied
    // -----------------------------------------------------------------------

    #[test]
    fn test_horse_blocked_by_leg() {
        let mut game = load();
        game.state.pieces.clear();

        // RED HORSE at [4,4]. Block the upward leg at [4,5].
        // This blocks steps [3,6] and [5,6] (which need leg [0,1] → [4,5]).
        insert(&mut game, vec![4, 4], "HORSE", "RED");
        insert(&mut game, vec![4, 5], "SOLDIER", "BLACK"); // blocks upward leg
        insert(&mut game, vec![4, 9], "GENERAL", "BLACK");
        // Shield the file so the generals do not face each other.
        insert(&mut game, vec![4, 5], "SOLDIER", "BLACK");
        insert(&mut game, vec![4, 0], "GENERAL", "RED");

        let dests = moves_for(&mut game, vec![4, 4]);

        // [3,6] and [5,6] require the leg at [4,5] — must be absent.
        assert!(
            !dests.contains(&vec![3u8, 6u8]),
            "HORSE must not reach [3,6] when leg [4,5] is blocked"
        );
        assert!(
            !dests.contains(&vec![5u8, 6u8]),
            "HORSE must not reach [5,6] when leg [4,5] is blocked"
        );

        // Horizontal legs are unblocked, so [2,3],[6,3],[2,5],[6,5] should be reachable.
        // (leg at [3,4] and [5,4] are clear)
        assert!(dests.contains(&vec![6u8, 5u8]), "HORSE must reach [6,5]");
        assert!(dests.contains(&vec![6u8, 3u8]), "HORSE must reach [6,3]");
    }

    // -----------------------------------------------------------------------
    // SOLDIER forward-only before the river, lateral moves after crossing
    // -----------------------------------------------------------------------

    #[test]
    fn test_soldier_forward_only_on_own_side() {
        let mut game = load();
        game.state.pieces.clear();

        // RED SOLDIER at [4,3] (own half, row 3).
        insert(&mut game, vec![4, 3], "SOLDIER", "RED");
        insert(&mut game, vec![4, 9], "GENERAL", "BLACK");
        // Shield the file so the generals do not face each other.
        insert(&mut game, vec![4, 5], "SOLDIER", "BLACK");
        insert(&mut game, vec![4, 0], "GENERAL", "RED");

        let dests = moves_for(&mut game, vec![4, 3]);

        // Forward [4,4] is valid.
        assert!(
            dests.contains(&vec![4u8, 4u8]),
            "SOLDIER must move forward to [4,4]"
        );

        // Lateral moves [3,3] and [5,3] must be absent (own side).
        assert!(
            !dests.contains(&vec![3u8, 3u8]),
            "SOLDIER must not move laterally on own side"
        );
        assert!(
            !dests.contains(&vec![5u8, 3u8]),
            "SOLDIER must not move laterally on own side"
        );

        // No backward move [4,2].
        assert!(
            !dests.contains(&vec![4u8, 2u8]),
            "SOLDIER must not move backward"
        );
    }

    #[test]
    fn test_soldier_gains_lateral_moves_after_crossing_river() {
        let mut game = load();
        game.state.pieces.clear();

        // RED SOLDIER already in enemy half at [4,6].
        insert(&mut game, vec![4, 6], "SOLDIER", "RED");
        insert(&mut game, vec![4, 9], "GENERAL", "BLACK");
        // Shield the file so the generals do not face each other.
        insert(&mut game, vec![4, 5], "SOLDIER", "BLACK");
        insert(&mut game, vec![4, 0], "GENERAL", "RED");

        let dests = moves_for(&mut game, vec![4, 6]);

        // Forward and both laterals must be available.
        assert!(
            dests.contains(&vec![4u8, 7u8]),
            "SOLDIER must still move forward in enemy half"
        );
        assert!(
            dests.contains(&vec![3u8, 6u8]),
            "SOLDIER must move left in enemy half"
        );
        assert!(
            dests.contains(&vec![5u8, 6u8]),
            "SOLDIER must move right in enemy half"
        );
    }

    // -----------------------------------------------------------------------
    // CANNON captures by jumping over exactly one screen piece
    // -----------------------------------------------------------------------

    #[test]
    fn test_cannon_captures_over_screen() {
        let mut game = load();
        game.state.pieces.clear();

        // RED CANNON at [0,5]. Screen at [0,7]. BLACK SOLDIER at [0,9].
        insert(&mut game, vec![0, 5], "CANNON", "RED");
        insert(&mut game, vec![0, 7], "SOLDIER", "RED"); // screen
        insert(&mut game, vec![0, 9], "SOLDIER", "BLACK"); // target
        insert(&mut game, vec![4, 9], "GENERAL", "BLACK");
        // Shield the file so the generals do not face each other.
        insert(&mut game, vec![4, 5], "SOLDIER", "BLACK");
        insert(&mut game, vec![4, 0], "GENERAL", "RED");

        let dests = moves_for(&mut game, vec![0, 5]);

        // [0,9] is a valid capture (one screen between cannon and target).
        assert!(
            dests.contains(&vec![0u8, 9u8]),
            "CANNON must capture [0,9] over the screen at [0,7]"
        );

        // [0,7] itself is not a valid move (it's occupied by an ally).
        assert!(
            !dests.contains(&vec![0u8, 7u8]),
            "CANNON must not land on the screen piece"
        );
    }

    // -----------------------------------------------------------------------
    // Flying-general rule: threat_only blueprint extends GENERAL's attack along
    // the full file, so leader_in_check returns true when generals face each other.
    // -----------------------------------------------------------------------

    #[test]
    fn test_flying_general_detected_by_threat_only_blueprint() {
        let mut game = load();
        game.state.pieces.clear();

        // RED GENERAL at [4,0], BLACK GENERAL at [4,9] — same file, nothing between.
        insert(&mut game, vec![4, 0], "GENERAL", "RED");
        insert(&mut game, vec![4, 9], "GENERAL", "BLACK");

        // It is RED's turn. BLACK's threat_only sliding blueprint extends from [4,9]
        // downward and must reach [4,0], putting RED in check.
        assert!(
            game.leader_in_check(),
            "Facing generals with no pieces between must be detected as check (flying-general rule)"
        );
    }

    #[test]
    fn test_flying_general_not_detected_when_shielded() {
        let mut game = load();
        game.state.pieces.clear();

        // Same setup but with a CHARIOT at [4,4] shielding on the file.
        insert(&mut game, vec![4, 0], "GENERAL", "RED");
        insert(&mut game, vec![4, 4], "CHARIOT", "RED");
        insert(&mut game, vec![4, 9], "GENERAL", "BLACK");

        // The CHARIOT blocks BLACK's sliding threat — RED should NOT be in check.
        assert!(
            !game.leader_in_check(),
            "Generals on same file but with a piece between must NOT be check"
        );
    }

    #[test]
    fn test_flying_general_triggers_checkmate_detection() {
        let mut game = load();
        game.state.pieces.clear();

        // RED GENERAL at [4,0], BLACK GENERAL at [4,9], no pieces between.
        // RED has only one piece (the GENERAL) and all its palace moves lead to
        // positions still on file 4 or to squares where BLACK still attacks.
        // Set up so RED truly has no legal escape: surround with BLACK cannons.
        insert(&mut game, vec![4, 0], "GENERAL", "RED");
        insert(&mut game, vec![4, 9], "GENERAL", "BLACK");
        // Block all palace moves for RED GENERAL:
        // [3,0],[5,0] — place BLACK cannons there (captures available but still facing general)
        // [4,1] — still on file 4, still facing general
        insert(&mut game, vec![3, 0], "CANNON", "BLACK");
        insert(&mut game, vec![5, 0], "CANNON", "BLACK");
        // [4,1] remains but after moving there RED is still on file 4 and still in check.
        // actual any_legal_moves check: [4,1] sim → RED at [4,1], BLACK GENERAL still at [4,9],
        // nothing between → still in check → not legal.

        // any_legal_moves must return false: RED has no safe square.
        assert!(
            !game.any_legal_moves(),
            "RED with all escapes cut off by flying-general must have no legal moves"
        );
    }

    // -----------------------------------------------------------------------
    // CANNON may use an enemy piece as its screen
    // -----------------------------------------------------------------------

    #[test]
    fn test_cannon_captures_over_enemy_screen() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![0, 5], "CANNON", "RED");
        insert(&mut game, vec![0, 7], "SOLDIER", "BLACK"); // enemy screen
        insert(&mut game, vec![0, 9], "CHARIOT", "BLACK"); // target
        insert(&mut game, vec![3, 0], "GENERAL", "RED");
        insert(&mut game, vec![5, 9], "GENERAL", "BLACK");

        let dests = moves_for(&mut game, vec![0, 5]);

        assert!(
            dests.contains(&vec![0u8, 9u8]),
            "CANNON must capture [0,9] over an enemy screen at [0,7]"
        );
        assert!(
            !dests.contains(&vec![0u8, 7u8]),
            "CANNON must not capture the screen itself"
        );
    }

    // -----------------------------------------------------------------------
    // The generals may not face each other: a general cannot step onto an open file
    // -----------------------------------------------------------------------

    #[test]
    fn test_general_cannot_step_onto_open_file() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![3, 0], "GENERAL", "RED");
        insert(&mut game, vec![4, 9], "GENERAL", "BLACK");

        let dests = moves_for(&mut game, vec![3, 0]);

        assert!(
            !dests.contains(&vec![4u8, 0u8]),
            "GENERAL must not step onto a file that is open to the enemy general"
        );
        assert!(
            dests.contains(&vec![3u8, 1u8]),
            "GENERAL may still step along its own file"
        );
    }

    #[test]
    fn test_general_may_step_onto_shielded_file() {
        let mut game = load();
        game.state.pieces.clear();

        insert(&mut game, vec![3, 0], "GENERAL", "RED");
        insert(&mut game, vec![4, 9], "GENERAL", "BLACK");
        insert(&mut game, vec![4, 5], "SOLDIER", "BLACK"); // shield

        let dests = moves_for(&mut game, vec![3, 0]);

        assert!(
            dests.contains(&vec![4u8, 0u8]),
            "GENERAL may step onto a file with a piece between the generals"
        );
    }

    #[test]
    fn test_piece_cannot_leave_generals_facing() {
        let mut game = load();
        game.state.pieces.clear();

        // RED CHARIOT is the only shield between the generals on file 4.
        insert(&mut game, vec![4, 0], "GENERAL", "RED");
        insert(&mut game, vec![4, 4], "CHARIOT", "RED");
        insert(&mut game, vec![4, 9], "GENERAL", "BLACK");

        let dests = moves_for(&mut game, vec![4, 4]);

        assert!(
            dests.iter().all(|d| d[0] == 4),
            "a pinned CHARIOT must stay on file 4, but could reach {:?}",
            dests
        );
    }
}
