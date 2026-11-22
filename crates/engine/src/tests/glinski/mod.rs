//! Gliński's hexagonal chess on the 91-cell hexagon.
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::shared::Position;
    use crate::specs::parse_game_spec;

    const FILES: &str = "abcdefghikl";

    /// A cell in Gliński notation: files a–l without j are `q = 0…10`, and rank `n` of file
    /// `q` is `r = max(0, q − 5) + n − 1`.
    fn cell(name: &str) -> Position {
        let q = FILES.find(&name[..1]).expect("a file letter") as u8;
        let rank: u8 = name[1..].parse().expect("a rank");
        pos![q, q.saturating_sub(5) + rank - 1]
    }

    fn load() -> Game {
        parse_game_spec("./specs/glinski.json")
            .map(Game::from_spec)
            .expect("Failed to load glinski spec")
    }

    /// A game with only the given pieces on the board, WHITE to move.
    fn empty_with(pieces: &[(&str, &str, &str)]) -> Game {
        let mut game = load();
        game.state.pieces.clear();
        for &(at, code, player) in pieces {
            game.state
                .pieces
                .insert(cell(at), Piece::new(code.to_string(), player.to_string()));
        }
        game
    }

    fn moves(game: &mut Game, from: &str) -> HashSet<Position> {
        game.transition(GameTransition::CalculateMoves {
            position: cell(from),
        })
        .unwrap();
        game.state
            .available_moves
            .as_ref()
            .map(|moves| moves.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn play(game: &mut Game, from: &str, to: &str) {
        moves(game, from);
        game.transition(GameTransition::ExecuteMove { position: cell(to) })
            .unwrap();
    }

    #[test]
    fn the_board_has_91_cells_and_36_pieces() {
        let game = load();
        assert_eq!(game.board.all_positions().len(), 91);
        assert_eq!(game.state.pieces.len(), 36);
    }

    #[test]
    fn starting_cells() {
        let game = load();
        let at = |name: &str| game.state.pieces.get(&cell(name)).cloned().expect(name);
        assert_eq!((at("g1").code, at("g1").player), ("KING".into(), "WHITE".into()));
        assert_eq!((at("e10").code, at("e10").player), ("QUEEN".into(), "BLACK".into()));
        assert_eq!((at("f5").code, at("f5").player), ("PAWN".into(), "WHITE".into()));
        assert_eq!((at("k7").code, at("k7").player), ("PAWN".into(), "BLACK".into()));
    }

    #[test]
    fn moves_from_the_centre() {
        for (code, expected) in [("ROOK", 30), ("BISHOP", 12), ("QUEEN", 42), ("KNIGHT", 12), ("KING", 12)] {
            let mut game = empty_with(&[("f6", code, "WHITE")]);
            assert_eq!(moves(&mut game, "f6").len(), expected, "{code}");
        }
    }

    #[test]
    fn bishops_start_on_three_colours() {
        let colours: HashSet<u8> = ["f1", "f2", "f3"]
            .iter()
            .map(|name| {
                let c = cell(name).coordinates().expect("a grid cell");
                (c[0] + c[1]) % 3
            })
            .collect();
        assert_eq!(colours.len(), 3);
    }

    #[test]
    fn a_pawn_double_steps_from_a_borrowed_start() {
        let mut from_own = empty_with(&[("c2", "PAWN", "WHITE")]);
        assert_eq!(moves(&mut from_own, "c2"), HashSet::from([cell("c3"), cell("c4")]));

        let mut from_another = empty_with(&[("e4", "PAWN", "WHITE")]);
        assert_eq!(moves(&mut from_another, "e4"), HashSet::from([cell("e5"), cell("e6")]));

        let mut elsewhere = empty_with(&[("e5", "PAWN", "WHITE")]);
        assert_eq!(moves(&mut elsewhere, "e5"), HashSet::from([cell("e6")]));
    }

    #[test]
    fn a_pawn_captures_at_sixty_degrees() {
        let mut game = empty_with(&[
            ("f5", "PAWN", "WHITE"),
            ("e5", "PAWN", "BLACK"),
            ("g5", "PAWN", "BLACK"),
            ("f6", "PAWN", "BLACK"),
        ]);
        assert_eq!(moves(&mut game, "f5"), HashSet::from([cell("e5"), cell("g5")]));
    }

    #[test]
    fn en_passant() {
        let mut game = empty_with(&[
            ("g1", "KING", "WHITE"),
            ("g10", "KING", "BLACK"),
            ("b5", "PAWN", "WHITE"),
            ("c7", "PAWN", "BLACK"),
        ]);
        game.state.current_turn = 1;
        play(&mut game, "c7", "c5");
        assert!(moves(&mut game, "b5").contains(&cell("c6")));
        game.transition(GameTransition::ExecuteMove { position: cell("c6") })
            .unwrap();
        assert!(game.state.pieces.get(&cell("c5")).is_none(), "the black pawn is taken");
        assert_eq!(game.state.pieces.get(&cell("c6")).map(|p| p.player.as_str()), Some("WHITE"));
    }

    #[test]
    fn promotion_on_the_bent_edge() {
        let mut game = empty_with(&[
            ("g1", "KING", "WHITE"),
            ("g10", "KING", "BLACK"),
            ("k6", "PAWN", "WHITE"),
        ]);
        play(&mut game, "k6", "k7");
        match &game.state.phase {
            GamePhase::Transforming { options, .. } => {
                let offered: HashSet<&str> = options.iter().map(String::as_str).collect();
                assert_eq!(offered, HashSet::from(["QUEEN", "ROOK", "BISHOP", "KNIGHT"]));
            }
            other => panic!("expected a promotion, got {other:?}"),
        }
    }

    #[test]
    fn stalemate_is_a_win() {
        let mut game = empty_with(&[
            ("d1", "QUEEN", "WHITE"),
            ("h9", "KING", "WHITE"),
            ("f11", "KING", "BLACK"),
        ]);
        assert!(!game.leader_in_check());
        play(&mut game, "d1", "e1");
        assert_eq!(
            game.state.phase,
            GamePhase::GameOver {
                winner: Some("WHITE".to_string())
            }
        );
    }
}
