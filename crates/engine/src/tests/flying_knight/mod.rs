//! The Flying Knight: White has the whole army and one move; Black has a king and the kingside
//! knight and two moves in a turn. White's king is in check when Black could take it within its
//! two moves; Black may step into check with the first of them if the second gets out.
#[cfg(test)]
mod tests {
    use crate::logic::{Answer, Game, GamePhase, Piece, Way};
    use crate::shared::Position;
    use crate::specs::parse_game_spec;
    use crate::tests::support::{go, moves_from, targets};

    const WHITE: &str = "WHITE";
    const BLACK: &str = "BLACK";

    fn at(place: [u8; 2]) -> Position {
        Position::from(format!("{},{}", place[0], place[1]))
    }

    fn start(spec: &str) -> Game {
        parse_game_spec(format!("./specs/{spec}.json"))
            .map(Game::from_spec)
            .expect("the spec reads")
    }

    /// `spec` with exactly `pieces` on the board and `mover` to play.
    fn position(spec: &str, mover: &str, pieces: &[(&str, &str, [u8; 2])]) -> Game {
        let mut game = start(spec);
        game.state.pieces.clear();
        for (player, code, place) in pieces {
            game.state.pieces.insert(
                at(*place),
                Piece::new((*code).to_string(), (*player).to_string()),
            );
        }
        game.state.current_turn = u8::from(mover == BLACK);
        game.work_out_ways();
        game
    }

    fn places(answers: &[[u8; 2]]) -> Vec<Answer> {
        answers
            .iter()
            .map(|place| Answer::Place(at(*place)))
            .collect()
    }

    fn has_way(ways: &[Way], answers: &[[u8; 2]]) -> bool {
        ways.iter().any(|way| way.answers == places(answers))
    }

    fn winner(game: &Game) -> Option<Option<String>> {
        match &game.state.phase {
            GamePhase::GameOver { winner } => Some(winner.clone()),
            GamePhase::Choosing { .. } => None,
        }
    }

    fn count(game: &Game, player: &str) -> usize {
        game.state
            .pieces
            .values()
            .filter(|piece| piece.player == player)
            .count()
    }

    #[test]
    fn white_has_the_army_and_black_a_king_and_the_kingside_knight() {
        let game = start("flying_knight");
        assert_eq!(count(&game, WHITE), 16);
        assert_eq!(count(&game, BLACK), 2);
        assert_eq!(
            game.piece_at_position(&at([4, 7])).map(|p| p.code.as_str()),
            Some("KING")
        );
        assert_eq!(
            game.piece_at_position(&at([6, 7])).map(|p| p.code.as_str()),
            Some("KNIGHT")
        );
        assert_eq!(game.current_player(), WHITE);
        assert_eq!(
            game.compute_ways().len(),
            20,
            "White's first move is chess's"
        );
    }

    #[test]
    fn black_takes_two_moves_in_a_turn() {
        let mut game = start("flying_knight");
        go(&mut game, at([4, 1])).unwrap();
        go(&mut game, at([4, 3])).unwrap(); // e2-e4
        assert_eq!(game.current_player(), BLACK);
        go(&mut game, at([6, 7])).unwrap();
        go(&mut game, at([5, 5])).unwrap(); // the knight to f6
        assert_eq!(game.current_player(), BLACK, "one move is not the turn");
        go(&mut game, at([4, 7])).unwrap();
        assert!(
            targets(&game).contains(&at([4, 6])),
            "the king's step is offered"
        );
        go(&mut game, at([4, 6])).unwrap(); // the king to e7
        assert_eq!(game.current_player(), WHITE);
        assert_eq!(game.history().len(), 2, "a turn of Black is one record");
    }

    #[test]
    fn black_may_step_into_check_with_the_first_move_and_out_with_the_second() {
        // The rook on a7 attacks the whole seventh rank: e7 is in check for the Black king.
        let game = position(
            "flying_knight",
            BLACK,
            &[
                (WHITE, "KING", [0, 0]),
                (WHITE, "ROOK", [0, 6]),
                (BLACK, "KING", [4, 7]),
                (BLACK, "KNIGHT", [6, 7]),
            ],
        );
        let ways = game.compute_ways();
        assert!(
            has_way(&ways, &[[4, 7], [4, 6], [4, 6], [5, 7]]),
            "the king steps onto e7 and out again, to f8"
        );
        assert!(
            !has_way(&ways, &[[4, 7], [4, 6], [6, 7], [5, 5]]),
            "a first move into check the knight's second move leaves in check is not a way"
        );
        assert!(
            ways.iter().all(|way| way.answers.len() == 4),
            "every way is two moves"
        );
    }

    #[test]
    fn black_may_not_end_its_turn_in_check() {
        let game = position(
            "flying_knight",
            BLACK,
            &[
                (WHITE, "KING", [0, 0]),
                (WHITE, "ROOK", [0, 6]),
                (BLACK, "KING", [4, 7]),
                (BLACK, "KNIGHT", [6, 7]),
            ],
        );
        // Once the king has stepped onto e7 (into the rook's check), the second move must be
        // the king's: any other piece's move leaves it in check.
        let ways = game.compute_ways();
        let after_e7: Vec<&Way> = ways
            .iter()
            .filter(|way| way.answers.starts_with(&places(&[[4, 7], [4, 6]])))
            .collect();
        assert!(!after_e7.is_empty());
        assert!(after_e7
            .iter()
            .all(|way| way.answers.get(2) == Some(&Answer::Place(at([4, 6])))));
    }

    /// A White king on e1 and a Black knight on g3, Black's king far away.
    fn king_and_knight(spec: &str) -> Game {
        position(
            spec,
            WHITE,
            &[
                (WHITE, "KING", [4, 0]),
                (BLACK, "KING", [7, 7]),
                (BLACK, "KNIGHT", [6, 2]),
            ],
        )
    }

    #[test]
    fn white_may_not_step_where_black_could_take_the_king_within_two_moves() {
        // From g3 the knight attacks f1 and e2 in one jump, and d2 and f2 only in two.
        let mut chess = king_and_knight("chess");
        assert_eq!(
            moves_from(&mut chess, at([4, 0])),
            vec![at([3, 0]), at([3, 1]), at([5, 1])],
            "in chess only f1 and e2 are attacked"
        );
        let mut flying = king_and_knight("flying_knight");
        assert_eq!(
            moves_from(&mut flying, at([4, 0])),
            vec![at([3, 0])],
            "d2 and f2 are two jumps from the knight: only d1 is safe"
        );
    }

    /// The king on e1 with its rook on h1, Black's knight on `knight`.
    fn castling(spec: &str, knight: [u8; 2]) -> Game {
        position(
            spec,
            WHITE,
            &[
                (WHITE, "KING", [4, 0]),
                (WHITE, "ROOK", [7, 0]),
                (BLACK, "KING", [4, 7]),
                (BLACK, "KNIGHT", knight),
            ],
        )
    }

    #[test]
    fn white_castles_unless_black_reaches_a_square_of_it_in_two() {
        // A knight on e4 reaches f1 in two jumps and attacks none of e1, f1, g1 in one.
        let mut chess = castling("chess", [4, 3]);
        assert!(
            moves_from(&mut chess, at([4, 0])).contains(&at([6, 0])),
            "chess castles"
        );
        let mut flying = castling("flying_knight", [4, 3]);
        assert!(
            !moves_from(&mut flying, at([4, 0])).contains(&at([6, 0])),
            "f1 is two jumps from e4: not through check"
        );
        let mut far = castling("flying_knight", [7, 7]);
        assert!(
            moves_from(&mut far, at([4, 0])).contains(&at([6, 0])),
            "a knight on h8 reaches none of them"
        );
    }

    fn ended(mover: &str, pieces: &[(&str, &str, [u8; 2])]) -> Option<Option<String>> {
        let mut game = position("flying_knight", mover, pieces);
        game.check_game_over();
        winner(&game)
    }

    #[test]
    fn black_stuck_in_check_loses() {
        // Rooks on a8, b7 and c6 hold ranks 8, 7 and 6: the king on h8 is in check, and two
        // steps take it no further than rank 6.
        let result = ended(
            BLACK,
            &[
                (WHITE, "KING", [0, 0]),
                (WHITE, "ROOK", [0, 7]),
                (WHITE, "ROOK", [1, 6]),
                (WHITE, "ROOK", [2, 5]),
                (BLACK, "KING", [7, 7]),
            ],
        );
        assert_eq!(result, Some(Some(WHITE.to_string())));
    }

    #[test]
    fn black_not_in_check_is_never_stuck_it_steps_out_and_back() {
        // The same queen and king as the mate, but with the rook-less queen on f7: Black is not
        // in check, and a king that may move twice can always step away and return.
        let result = ended(
            BLACK,
            &[
                (WHITE, "KING", [0, 0]),
                (WHITE, "QUEEN", [5, 6]),
                (BLACK, "KING", [7, 7]),
            ],
        );
        assert_eq!(result, None, "the game goes on: Black has a way");
    }

    #[test]
    fn white_stuck_in_a_two_move_check_loses() {
        // The knight on b3 attacks a1, and a2 and b1 are each one more jump away; the pawn on b2,
        // blocked by the knight, has no move but keeps White's material sufficient.
        let result = ended(
            WHITE,
            &[
                (WHITE, "KING", [0, 0]),
                (WHITE, "PAWN", [1, 1]),
                (BLACK, "KING", [7, 7]),
                (BLACK, "KNIGHT", [1, 2]),
            ],
        );
        assert_eq!(result, Some(Some(BLACK.to_string())));
    }

    #[test]
    fn white_stuck_but_not_in_a_two_move_check_draws() {
        // The knight on c3 attacks a2 and b1, and reaches b2 in two; a1 it needs four jumps for.
        let result = ended(
            WHITE,
            &[
                (WHITE, "KING", [0, 0]),
                (WHITE, "PAWN", [7, 6]),
                (BLACK, "KING", [7, 7]),
                (BLACK, "KNIGHT", [2, 2]),
            ],
        );
        assert_eq!(result, Some(None));
    }
}
