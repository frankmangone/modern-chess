#[cfg(test)]
mod tests {
    use crate::logic::{Game, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    /// Castling geometry of every shipped spec that has it, written out by hand:
    /// (spec file, king file, rook files, king travel in squares, board width, board height).
    const CASTLING_SPECS: [(&str, u8, [u8; 2], u8, u8, u8); 7] = [
        ("chess", 4, [0, 7], 2, 8, 8),
        ("atomic", 4, [0, 7], 2, 8, 8),
        ("crazyhouse", 4, [0, 7], 2, 8, 8),
        ("king_of_the_hill", 4, [0, 7], 2, 8, 8),
        ("three_check", 4, [0, 7], 2, 8, 8),
        ("capablanca", 5, [0, 9], 2, 10, 10),
        ("gothic", 5, [0, 9], 3, 10, 8),
    ];

    struct Scenario {
        spec: &'static str,
        player: &'static str,
        home_rank: u8,
        enemy_rank: u8,
        king_file: u8,
        rook_file: u8,
        landing: u8,
    }

    /// Every (spec, player, side) combination, with the landing file worked out from the table.
    fn scenarios() -> Vec<Scenario> {
        let mut out = Vec::new();
        for (spec, king_file, rook_files, travel, _width, height) in CASTLING_SPECS {
            for (player, home_rank, enemy_rank) in
                [("WHITE", 0u8, height - 1), ("BLACK", height - 1, 0u8)]
            {
                for rook_file in rook_files {
                    let landing = if rook_file < king_file {
                        king_file - travel
                    } else {
                        king_file + travel
                    };
                    out.push(Scenario {
                        spec,
                        player,
                        home_rank,
                        enemy_rank,
                        king_file,
                        rook_file,
                        landing,
                    });
                }
            }
        }
        out
    }

    /// A game holding only the castling king and one rook of the mover, plus a far-away enemy
    /// king. `tweak` can add a blocker or change the rook before the moves are calculated.
    fn setup(s: &Scenario, tweak: impl FnOnce(&mut Game)) -> Game {
        let path = format!("./specs/{}.json", s.spec);
        let mut game = parse_game_spec(&path)
            .map(Game::from_spec)
            .unwrap_or_else(|e| panic!("{path} should parse: {e:?}"));
        game.state.pieces.clear();
        let enemy = if s.player == "WHITE" { "BLACK" } else { "WHITE" };
        let put = |g: &mut Game, x: u8, y: u8, code: &str, owner: &str| {
            g.state
                .pieces
                .insert(vec![x, y], Piece::new(code.to_string(), owner.to_string()));
        };
        put(&mut game, s.king_file, s.home_rank, "KING", s.player);
        put(&mut game, s.rook_file, s.home_rank, "ROOK", s.player);
        put(&mut game, s.king_file, s.enemy_rank, "KING", enemy);
        game.state.current_turn = game
            .turn_order
            .iter()
            .position(|p| p == s.player)
            .expect("player should be in the turn order") as u8;
        tweak(&mut game);
        game
    }

    fn castle_is_offered(game: &mut Game, s: &Scenario) -> bool {
        game.transition(GameTransition::CalculateMoves {
            position: vec![s.king_file, s.home_rank],
        })
        .expect("selecting the king should succeed");
        game.state
            .available_moves
            .as_ref()
            .map_or(false, |m| m.contains_key(&vec![s.landing, s.home_rank]))
    }

    fn describe(s: &Scenario) -> String {
        format!(
            "{} {} king e={} rook={} landing={}",
            s.spec, s.player, s.king_file, s.rook_file, s.landing
        )
    }

    #[test]
    fn test_castling_is_offered_on_both_sides_for_both_players() {
        for s in scenarios() {
            let mut game = setup(&s, |_| {});
            assert!(castle_is_offered(&mut game, &s), "should castle: {}", describe(&s));
        }
    }

    /// Real castling needs every square between the king and the rook to be empty, including
    /// the square next to the rook that the king never crosses.
    #[test]
    fn test_any_piece_between_king_and_rook_prevents_castling() {
        for s in scenarios() {
            let (lo, hi) = (s.king_file.min(s.rook_file), s.king_file.max(s.rook_file));
            for blocker_file in (lo + 1)..hi {
                let mut game = setup(&s, |g| {
                    g.state.pieces.insert(
                        vec![blocker_file, s.home_rank],
                        Piece::new("KNIGHT".to_string(), s.player.to_string()),
                    );
                });
                assert!(
                    !castle_is_offered(&mut game, &s),
                    "blocker on file {blocker_file} should prevent castling: {}",
                    describe(&s)
                );
            }
        }
    }

    #[test]
    fn test_cannot_castle_without_a_rook() {
        for s in scenarios() {
            let mut game = setup(&s, |g| {
                g.state.pieces.remove(&vec![s.rook_file, s.home_rank]);
            });
            assert!(
                !castle_is_offered(&mut game, &s),
                "no rook, no castling: {}",
                describe(&s)
            );
        }
    }

    #[test]
    fn test_cannot_castle_with_a_rook_that_has_moved() {
        for s in scenarios() {
            let mut game = setup(&s, |g| {
                g.state
                    .pieces
                    .get_mut(&vec![s.rook_file, s.home_rank])
                    .expect("rook was placed")
                    .total_moves = 1;
            });
            assert!(
                !castle_is_offered(&mut game, &s),
                "a rook that has moved cannot castle: {}",
                describe(&s)
            );
        }
    }

    /// Castling also moves the rook, to the square the king crossed last.
    #[test]
    fn test_castling_moves_the_rook_next_to_the_king() {
        for s in scenarios() {
            let mut game = setup(&s, |_| {});
            assert!(castle_is_offered(&mut game, &s));
            game.transition(GameTransition::ExecuteMove {
                position: vec![s.landing, s.home_rank],
            })
            .unwrap_or_else(|e| panic!("castling should execute ({}): {e:?}", describe(&s)));

            let rook_file = if s.rook_file < s.king_file { s.landing + 1 } else { s.landing - 1 };
            let at = |x: u8| game.state.pieces.get(&vec![x, s.home_rank]).map(|p| p.code.as_str());
            assert_eq!(at(s.landing), Some("KING"), "{}", describe(&s));
            assert_eq!(at(rook_file), Some("ROOK"), "{}", describe(&s));
            assert_eq!(at(s.rook_file), None, "rook must leave its corner: {}", describe(&s));
        }
    }
}
