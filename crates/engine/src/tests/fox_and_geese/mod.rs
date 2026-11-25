//! Fox and Geese: thirteen geese against a fox on a cross of 33 points. The geese step forward or
//! sideways and never take; the fox steps any way along the lines and takes a goose by jumping it,
//! and must go on jumping while it can. The geese win by leaving the fox no move, the fox by
//! taking enough that the geese cannot, and a position seen three times is a draw.
#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, Piece};
    use crate::shared::Position;
    use crate::specs::parse_game_spec;
    use crate::tests::support::{go, jumping, moves_from, play};

    const GEESE: &str = "GEESE";
    const FOX: &str = "FOX";

    fn at(place: [u8; 2]) -> Position {
        Position::from(format!("{},{}", place[0], place[1]))
    }

    fn start() -> Game {
        parse_game_spec("./specs/fox_and_geese.json")
            .map(Game::from_spec)
            .expect("the spec reads")
    }

    /// The game with exactly these geese and this fox on the board and `mover` to play.
    fn position(mover: &str, fox: [u8; 2], geese: &[[u8; 2]]) -> Game {
        let mut game = start();
        game.state.pieces.clear();
        game.state
            .pieces
            .insert(at(fox), Piece::new("FOX".to_string(), FOX.to_string()));
        for place in geese {
            game.state.pieces.insert(
                at(*place),
                Piece::new("GOOSE".to_string(), GEESE.to_string()),
            );
        }
        game.state.current_turn = u8::from(mover == FOX);
        game.work_out_ways();
        game
    }

    fn count(game: &Game, player: &str) -> usize {
        game.state
            .pieces
            .values()
            .filter(|piece| piece.player == player)
            .count()
    }

    fn winner(game: &Game) -> Option<Option<String>> {
        match &game.state.phase {
            GamePhase::GameOver { winner } => Some(winner.clone()),
            GamePhase::Choosing { .. } => None,
        }
    }

    fn sorted(mut places: Vec<Position>) -> Vec<Position> {
        places.sort();
        places
    }

    /// Four geese out of the way, so that taking one more is not the end of the game.
    const SPARE: [[u8; 2]; 4] = [[2, 0], [3, 0], [4, 0], [2, 1]];

    #[test]
    fn the_geese_start_in_the_three_bottom_rows_and_the_fox_in_the_middle() {
        let game = start();
        assert_eq!(count(&game, GEESE), 13);
        assert_eq!(count(&game, FOX), 1);
        assert_eq!(game.current_player(), GEESE);
        assert_eq!(
            game.piece_at_position(&at([3, 3])).map(|p| p.code.as_str()),
            Some("FOX")
        );
        for y in 0..3 {
            let row = (0..7u8)
                .filter(|&x| game.piece_at_position(&at([x, y])).is_some())
                .count();
            assert_eq!(row, if y == 2 { 7 } else { 3 }, "row {y}");
        }
    }

    #[test]
    fn the_board_is_a_cross_of_33_points() {
        let spec = parse_game_spec("./specs/fox_and_geese.json").unwrap();
        assert_eq!(49 - spec.board.disabled_positions.len(), 33);
        let game = start();
        for corner in [
            [0, 0],
            [1, 1],
            [5, 0],
            [6, 6],
            [0, 5],
            [1, 6],
            [6, 1],
            [5, 5],
        ] {
            assert!(
                game.board.node(&at(corner)).is_none(),
                "{corner:?} is not on the board"
            );
        }
    }

    #[test]
    fn a_goose_steps_forward_or_sideways_and_never_back_or_across() {
        let mut game = position(GEESE, [6, 4], &[[3, 3], [2, 0], [3, 0], [4, 0]]);
        assert_eq!(
            sorted(moves_from(&mut game, at([3, 3]))),
            vec![at([2, 3]), at([3, 4]), at([4, 3])]
        );
    }

    #[test]
    fn a_goose_does_not_jump_the_fox() {
        let mut game = position(GEESE, [3, 4], &[[3, 3], [2, 0], [3, 0], [4, 0]]);
        assert_eq!(
            sorted(moves_from(&mut game, at([3, 3]))),
            vec![at([2, 3]), at([4, 3])],
            "the fox in front is not taken and not jumped"
        );
    }

    #[test]
    fn the_fox_steps_one_point_along_the_lines_and_not_across_them() {
        let mut game = position(FOX, [3, 3], &SPARE);
        game.state.current_turn = 1;
        game.work_out_ways();
        assert_eq!(
            sorted(moves_from(&mut game, at([3, 3]))),
            vec![at([2, 3]), at([3, 2]), at([3, 4]), at([4, 3])]
        );
    }

    #[test]
    fn the_fox_takes_a_goose_by_jumping_it_to_the_empty_point_behind() {
        let mut geese = SPARE.to_vec();
        geese.push([3, 4]);
        let mut game = position(FOX, [3, 3], &geese);
        let targets = moves_from(&mut game, at([3, 3]));
        assert!(targets.contains(&at([3, 5])), "{targets:?}");
        go(&mut game, at([3, 5])).unwrap();
        assert!(
            game.piece_at_position(&at([3, 4])).is_none(),
            "the goose is gone"
        );
        assert_eq!(count(&game, GEESE), 4);
        assert_eq!(game.current_player(), GEESE);
    }

    #[test]
    fn the_fox_cannot_jump_a_goose_with_something_behind_it_or_two_in_a_row() {
        let mut geese = SPARE.to_vec();
        geese.extend([[3, 4], [3, 5]]);
        let mut game = position(FOX, [3, 3], &geese);
        let targets = moves_from(&mut game, at([3, 3]));
        assert!(!targets.contains(&at([3, 5])) && !targets.contains(&at([3, 6])));
    }

    #[test]
    fn the_fox_cannot_jump_off_the_cross() {
        let mut geese = SPARE.to_vec();
        geese.push([0, 2]);
        let mut game = position(FOX, [0, 3], &geese);
        let targets = moves_from(&mut game, at([0, 3]));
        assert!(
            !targets.contains(&at([0, 1])),
            "(0,1) is a corner that is not on the board"
        );
        assert_eq!(sorted(targets), vec![at([0, 4]), at([1, 3])]);
    }

    #[test]
    fn the_fox_goes_on_jumping_and_only_with_the_same_piece() {
        let geese = [
            [3, 3],
            [4, 2],
            [2, 1],
            [3, 1],
            [4, 1],
            [3, 0],
            [2, 0],
            [4, 0],
        ];
        let mut game = position(FOX, [3, 4], &geese);
        play(&mut game, at([3, 4]), at([3, 2])).unwrap();
        assert_eq!(jumping(&game), Some(at([3, 2])), "the fox is mid-jump");
        assert_eq!(game.current_player(), FOX, "the turn has not passed");
        assert_eq!(
            sorted(moves_from(&mut game, at([3, 2]))),
            vec![at([5, 2])],
            "only a jump goes on, and the other geese cannot be reached"
        );
        go(&mut game, at([5, 2])).unwrap();
        assert_eq!(count(&game, GEESE), 6);
        assert_eq!(game.current_player(), GEESE);
        assert_eq!(jumping(&game), None);
    }

    #[test]
    fn the_geese_win_when_the_fox_has_no_move() {
        // The fox in the corner of the top arm: a goose beside it and one below, each with a goose
        // behind it, and the hole at (1,6) on its other side.
        let geese = [[3, 6], [4, 6], [2, 5], [2, 4], [3, 4]];
        let mut game = position(FOX, [2, 6], &geese);
        game.check_game_over();
        assert_eq!(winner(&game), Some(Some(GEESE.to_string())));
    }

    #[test]
    fn the_fox_is_not_trapped_while_it_can_jump_out() {
        // Beside the fox, a goose with an empty point behind it: not trapped.
        let geese = [[3, 6], [2, 5], [4, 6]];
        let mut game = position(FOX, [2, 6], &geese);
        game.check_game_over();
        assert_eq!(winner(&game), None);
    }

    #[test]
    fn the_fox_wins_when_too_few_geese_are_left_to_trap_it() {
        let mut game = position(FOX, [3, 3], &[[3, 4], [2, 0], [3, 0], [4, 0]]);
        play(&mut game, at([3, 3]), at([3, 5])).unwrap();
        assert_eq!(count(&game, GEESE), 3);
        assert_eq!(winner(&game), Some(Some(FOX.to_string())));
    }

    #[test]
    fn four_geese_are_still_a_game() {
        let mut game = position(FOX, [3, 3], &[[3, 4], [2, 0], [3, 0], [4, 0], [2, 1]]);
        play(&mut game, at([3, 3]), at([3, 5])).unwrap();
        assert_eq!(count(&game, GEESE), 4);
        assert_eq!(winner(&game), None);
    }

    #[test]
    fn geese_with_no_move_lose() {
        // Every goose in the top arm, the three in the top row against the edge and the others
        // behind them: no goose can step forward or sideways.
        let geese = [[2, 6], [3, 6], [4, 6]];
        let mut game = position(GEESE, [3, 3], &geese);
        game.check_game_over();
        assert_eq!(winner(&game), Some(Some(FOX.to_string())));
    }

    #[test]
    fn a_position_seen_three_times_is_a_draw() {
        let mut game = position(GEESE, [6, 4], &[[2, 2], [2, 1], [3, 1], [4, 1], [4, 0]]);
        let there_and_back = [
            ([2, 2], [1, 2]),
            ([6, 4], [6, 3]),
            ([1, 2], [2, 2]),
            ([6, 3], [6, 4]),
        ];
        for round in 0..2 {
            for (from, to) in there_and_back {
                play(&mut game, at(from), at(to)).unwrap();
                assert_eq!(winner(&game), None, "round {round}");
            }
        }
        // The third time round, the position after the first turn comes up a third time.
        play(&mut game, at([2, 2]), at([1, 2])).unwrap();
        assert_eq!(winner(&game), Some(None));
    }
}
