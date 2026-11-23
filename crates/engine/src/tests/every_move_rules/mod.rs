//! The rules that look at every move, written as blocks in the specs: check (a king may not be
//! left attacked), Atomic's own-king rule, Racing Kings' ban on checking the opponent, and the
//! refusal of the old keys they replace.
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::{json, Value};

    use crate::logic::{Game, Piece};
    use crate::shared::Position;
    use crate::specs::{parse_game_spec, parse_game_spec_str};
    use crate::tests::support::{pick, targets};

    /// A shipped game with its pieces taken off, so each test places every piece.
    fn empty(spec: &str) -> Game {
        let mut game = parse_game_spec(format!("./specs/{spec}.json"))
            .map(Game::from_spec)
            .unwrap_or_else(|error| panic!("{spec} loads: {error:?}"));
        game.state.pieces.clear();
        game
    }

    fn put(game: &mut Game, at: [u8; 2], code: &str, player: &str) {
        game.state.pieces.insert(
            Position::from_coordinates(&at),
            Piece::new(code.to_string(), player.to_string()),
        );
    }

    fn square(at: [u8; 2]) -> Position {
        Position::from_coordinates(&at)
    }

    /// The squares offered to the player to move for the piece at `from`.
    fn offered(game: &mut Game, from: [u8; 2]) -> BTreeSet<Position> {
        pick(game, square(from)).expect("the piece can be picked");
        targets(game).into_iter().collect()
    }

    #[test]
    fn a_pinned_piece_may_not_leave_the_line() {
        let mut game = empty("chess");
        put(&mut game, [4, 0], "KING", "WHITE");
        put(&mut game, [4, 1], "ROOK", "WHITE");
        put(&mut game, [4, 7], "ROOK", "BLACK");
        put(&mut game, [0, 7], "KING", "BLACK");

        let moves = offered(&mut game, [4, 1]);
        assert!(moves.contains(&square([4, 7])), "taking the pinner is fine");
        assert!(!moves.contains(&square([5, 1])), "leaving the file is not");
    }

    #[test]
    fn the_king_may_not_step_into_attack() {
        let mut game = empty("chess");
        put(&mut game, [4, 0], "KING", "WHITE");
        put(&mut game, [3, 7], "ROOK", "BLACK");
        put(&mut game, [7, 7], "KING", "BLACK");

        let moves = offered(&mut game, [4, 0]);
        assert!(!moves.contains(&square([3, 0])), "the rook covers the file");
        assert!(moves.contains(&square([5, 0])));
    }

    #[test]
    fn a_move_that_answers_a_check_is_offered_and_others_are_not() {
        let mut game = empty("chess");
        put(&mut game, [4, 0], "KING", "WHITE");
        put(&mut game, [0, 3], "ROOK", "WHITE");
        put(&mut game, [4, 7], "ROOK", "BLACK");
        put(&mut game, [7, 7], "KING", "BLACK");

        assert_eq!(offered(&mut game, [0, 3]), BTreeSet::from([square([4, 3])]));
    }

    #[test]
    fn taking_the_enemy_king_is_allowed_even_if_ones_own_king_is_attacked() {
        let mut game = empty("chess");
        put(&mut game, [4, 0], "KING", "WHITE");
        put(&mut game, [4, 7], "ROOK", "BLACK");
        put(&mut game, [0, 7], "ROOK", "WHITE");
        put(&mut game, [1, 7], "KING", "BLACK");

        let moves = offered(&mut game, [0, 7]);
        assert!(moves.contains(&square([1, 7])), "the king can be taken");
        assert!(
            !moves.contains(&square([0, 6])),
            "a quiet move still leaves check"
        );
    }

    #[test]
    fn check_rule_ignores_a_side_with_no_king() {
        let mut game = empty("chess");
        put(&mut game, [0, 0], "ROOK", "WHITE");
        put(&mut game, [7, 7], "KING", "BLACK");

        assert_eq!(
            offered(&mut game, [0, 0]).len(),
            14,
            "every rook move is offered"
        );
    }

    /// WHITE's king on 3,0 is attacked by BLACK's queen on 3,7; the other players' kings stand
    /// out of the way.
    fn four_player_check() -> Game {
        let mut game = empty("4player_chess");
        put(&mut game, [3, 0], "KING", "WHITE");
        put(&mut game, [3, 7], "QUEEN", "BLACK");
        put(&mut game, [7, 7], "KING", "BLACK");
        put(&mut game, [1, 5], "KING", "SILVER");
        put(&mut game, [7, 4], "KING", "GOLD");
        game
    }

    #[test]
    fn four_player_check_filters_like_before() {
        let mut game = four_player_check();
        put(&mut game, [0, 3], "QUEEN", "WHITE");

        let moves = offered(&mut game, [0, 3]);
        assert_eq!(moves, BTreeSet::from([square([3, 3]), square([3, 6])]));
    }

    #[test]
    fn taking_any_one_enemy_king_in_four_player_is_enough() {
        let mut game = four_player_check();
        game.state.pieces.remove(&square([1, 5]));
        put(&mut game, [0, 5], "KING", "SILVER");
        put(&mut game, [0, 7], "QUEEN", "WHITE");

        let moves = offered(&mut game, [0, 7]);
        assert!(
            moves.contains(&square([0, 5])),
            "SILVER's king can be taken"
        );
        assert!(
            !moves.contains(&square([0, 6])),
            "a quiet move leaves WHITE in check"
        );
    }

    #[test]
    fn atomic_a_capture_that_blows_up_ones_own_king_is_refused() {
        let mut game = empty("atomic");
        put(&mut game, [4, 3], "KING", "WHITE");
        put(&mut game, [4, 4], "ROOK", "BLACK");
        put(&mut game, [7, 7], "KING", "BLACK");
        put(&mut game, [4, 5], "ROOK", "WHITE");

        let moves = offered(&mut game, [4, 5]);
        assert!(
            !moves.contains(&square([4, 4])),
            "the blast would reach WHITE's own king"
        );
    }

    #[test]
    fn atomic_kings_may_stand_side_by_side() {
        let mut game = empty("atomic");
        put(&mut game, [4, 3], "KING", "WHITE");
        put(&mut game, [4, 4], "KING", "BLACK");
        put(&mut game, [0, 3], "ROOK", "BLACK");
        put(&mut game, [7, 1], "PAWN", "WHITE");

        assert!(offered(&mut game, [7, 1]).contains(&square([7, 2])));
    }

    /// Racing Kings with a WHITE rook that may step to a square that checks BLACK's king.
    fn racing_kings() -> Game {
        let mut game = empty("racing_kings");
        put(&mut game, [0, 0], "KING", "WHITE");
        put(&mut game, [3, 3], "ROOK", "WHITE");
        put(&mut game, [7, 7], "KING", "BLACK");
        game
    }

    #[test]
    fn racing_kings_a_move_that_checks_the_opponent_is_refused() {
        let mut game = racing_kings();
        assert!(!offered(&mut game, [3, 3]).contains(&square([7, 3])));
    }

    #[test]
    fn racing_kings_a_move_that_does_not_is_offered() {
        let mut game = racing_kings();
        assert!(offered(&mut game, [3, 3]).contains(&square([5, 3])));
    }

    #[test]
    fn no_capture_means_quiet_moves_are_offered() {
        let mut game = empty("checkers");
        put(&mut game, [3, 3], "MAN", "RED");
        put(&mut game, [6, 6], "MAN", "BLACK");

        let moves = offered(&mut game, [3, 3]);
        assert!(
            moves.contains(&square([4, 4])),
            "a quiet step is offered: {moves:?}"
        );
    }

    #[test]
    fn a_capture_exists_so_only_captures_are_offered() {
        let mut game = empty("checkers");
        put(&mut game, [3, 3], "MAN", "RED");
        put(&mut game, [4, 4], "MAN", "BLACK");
        put(&mut game, [0, 0], "MAN", "RED");

        let jumping = offered(&mut game, [3, 3]);
        assert_eq!(jumping, BTreeSet::from([square([5, 5])]));
        game.state.ways = None;
        game.work_out_ways();
        assert!(
            pick(&mut game, square([0, 0])).is_err() || targets(&game).is_empty(),
            "a man with only a quiet move is not offered while a capture exists"
        );
    }

    #[test]
    fn a_player_who_is_out_has_no_king_to_take_in_four_player() {
        // SILVER is checkmated and out of the turn order, but its king stays on the board.
        // Taking it does not answer WHITE's own check.
        let mut game = four_player_check();
        game.state.pieces.remove(&square([1, 5]));
        put(&mut game, [0, 5], "KING", "SILVER");
        put(&mut game, [0, 7], "QUEEN", "WHITE");
        game.state.turn_order.retain(|player| player != "SILVER");

        let moves = offered(&mut game, [0, 7]);
        assert!(!moves.contains(&square([0, 5])), "{moves:?}");
        assert!(!moves.contains(&square([0, 6])), "{moves:?}");
    }

    #[test]
    fn a_mid_jump_must_entry_another_piece_meets_does_not_end_the_turn() {
        // RED's man on 4,4 has jumped and may jump again. A second `must` entry (take a king)
        // can only be met by RED's king elsewhere; the jump lock comes first, so the man's
        // jump stays and the turn does not end.
        let mut spec: Value = serde_json::from_str(
            &std::fs::read_to_string("./specs/checkers.json").expect("the spec reads"),
        )
        .expect("the spec is JSON");
        spec["must"] = json!([
            { "test": "REMOVED", "where": { "owner": "ENEMY" } },
            { "test": "REMOVED", "where": { "owner": "ENEMY", "codes": ["KING"] } }
        ]);
        let mut game = Game::from_spec(parse_game_spec_str(&spec.to_string()).expect("reads"));
        game.state.pieces.clear();
        put(&mut game, [4, 4], "MAN", "RED");
        put(&mut game, [5, 5], "MAN", "BLACK");
        put(&mut game, [0, 0], "KING", "RED");
        put(&mut game, [1, 1], "KING", "BLACK");
        game.state.stage = Some(crate::logic::StageState {
            name: crate::shared::Symbol::new("JUMPING"),
            last: Some(square([4, 4])),
        });

        let ways = game.compute_ways();
        assert_eq!(ways.len(), 1, "{ways:?}");
        assert_eq!(
            ways[0].answers.first(),
            Some(&crate::logic::Answer::Place(square([4, 4])))
        );
    }
}
