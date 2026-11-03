#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::logic::{Game, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    /// Chess rules on an otherwise empty board, so each test controls every piece.
    fn empty_chess() -> Game {
        let mut game = parse_game_spec("./specs/chess.json")
            .map(Game::from_spec)
            .expect("Failed to load chess spec");
        game.state.pieces.clear();
        game
    }

    fn insert(game: &mut Game, pos: [u8; 2], code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos.to_vec(), Piece::new(code.to_string(), player.to_string()));
    }

    /// Squares offered to the current player for the piece at `from`.
    fn offered_moves(game: &mut Game, from: [u8; 2]) -> BTreeSet<Vec<u8>> {
        game.transition(GameTransition::CalculateMoves {
            position: from.to_vec(),
        })
        .expect("selecting the piece should succeed");
        game.state
            .available_moves
            .as_ref()
            .expect("moving phase should expose available moves")
            .keys()
            .cloned()
            .collect()
    }

    /// WHITE rook on [4,1] shields its king on [4,0] from a BLACK rook on [4,7].
    /// Stepping off the file would expose the king, so only moves along the file are legal,
    /// including capturing the pinning rook.
    #[test]
    fn test_pinned_piece_can_only_move_along_the_pin() {
        let mut game = empty_chess();
        insert(&mut game, [4, 0], "KING", "WHITE");
        insert(&mut game, [4, 1], "ROOK", "WHITE");
        insert(&mut game, [4, 7], "ROOK", "BLACK");
        insert(&mut game, [0, 7], "KING", "BLACK");

        let moves = offered_moves(&mut game, [4, 1]);

        let expected: BTreeSet<Vec<u8>> = [
            [4, 2],
            [4, 3],
            [4, 4],
            [4, 5],
            [4, 6],
            [4, 7], // captures the pinning rook
        ]
        .iter()
        .map(|p| p.to_vec())
        .collect();
        assert_eq!(moves, expected);
    }

    /// WHITE king on [4,0] is checked by a BLACK rook on [4,7]. A WHITE rook on [0,3] may
    /// only play the move that blocks the file; every other rook move leaves the king in check.
    #[test]
    fn test_in_check_only_moves_that_resolve_the_check_are_offered() {
        let mut game = empty_chess();
        insert(&mut game, [4, 0], "KING", "WHITE");
        insert(&mut game, [0, 3], "ROOK", "WHITE");
        insert(&mut game, [4, 7], "ROOK", "BLACK");
        insert(&mut game, [7, 7], "KING", "BLACK");

        let moves = offered_moves(&mut game, [0, 3]);

        let expected: BTreeSet<Vec<u8>> = [vec![4u8, 3u8]].into_iter().collect();
        assert_eq!(moves, expected);
    }

    /// A piece with nothing to shield must keep all of its moves.
    #[test]
    fn test_unpinned_piece_keeps_all_moves() {
        let mut game = empty_chess();
        insert(&mut game, [4, 0], "KING", "WHITE");
        insert(&mut game, [0, 3], "ROOK", "WHITE");
        insert(&mut game, [7, 7], "KING", "BLACK");

        let moves = offered_moves(&mut game, [0, 3]);

        // 7 squares along row 3 (x = 1..=7), plus 4 up the file (y = 4..=7) and 3 down (y = 0..=2).
        assert_eq!(moves.len(), 14);
    }

    /// No shipped spec may start with a player already in check. Every spec is loaded and each
    /// player's leader is tested in turn, so a layout like a queen aimed at a king is caught.
    #[test]
    fn test_no_shipped_spec_starts_with_a_player_in_check() {
        let mut offenders = Vec::new();
        let specs = std::fs::read_dir("./specs").expect("specs directory should exist");
        for entry in specs {
            let path = entry.expect("readable entry").path();
            let spec = parse_game_spec(path.to_str().expect("utf-8 path"))
                .unwrap_or_else(|e| panic!("{} should parse: {e:?}", path.display()));
            let mut game = Game::from_spec(spec);
            for turn in 0..game.turn_order.len() {
                game.state.current_turn = turn as u8;
                if game.leader_in_check() {
                    offenders.push(format!("{} ({})", path.display(), game.turn_order[turn]));
                }
            }
        }
        assert!(offenders.is_empty(), "players starting in check: {offenders:?}");
    }
}
