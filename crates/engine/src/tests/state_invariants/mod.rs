#[cfg(test)]
mod tests {
    use crate::logic::{Answer, Game, Piece};
    use crate::shared::Position;
    use crate::specs::parse_game_spec;
    use crate::tests::support::{
        chosen, drop_squares, go, phase_is_choosing, pick, pick_hand, promote, promotion_options,
        remaining, targets,
    };

    fn load_side_effects_game() -> Game {
        parse_game_spec("./src/tests/side_effects/spec.json")
            .map(Game::from_spec)
            .expect("Failed to load side_effects test spec")
    }

    fn load_shogi() -> Game {
        parse_game_spec("./specs/shogi.json")
            .map(Game::from_spec)
            .expect("Failed to load shogi spec")
    }

    fn load_chess() -> Game {
        parse_game_spec("./specs/chess.json")
            .map(Game::from_spec)
            .expect("Failed to load chess spec")
    }

    fn insert(game: &mut Game, pos: Position, code: &str, player: &str) {
        game.state
            .pieces
            .insert(pos, Piece::new(code.to_string(), player.to_string()));
    }

    /// The answers of the ways the game keeps for the turn.
    fn kept(game: &Game) -> Option<Vec<Vec<Answer>>> {
        let ways = game.state.ways.as_ref()?;
        Some(ways.iter().map(|way| way.answers.clone()).collect())
    }

    /// Whether the ways the game keeps are the ones of the player to move, as worked out from
    /// the board now: nothing stale from a turn that is over.
    fn ways_are_fresh(game: &Game) -> bool {
        let fresh: Vec<Vec<Answer>> = game
            .compute_ways()
            .into_iter()
            .map(|way| way.answers)
            .collect();
        kept(game) == Some(fresh)
    }

    #[test]
    fn a_picked_piece_keeps_the_turns_ways() {
        let mut game = load_side_effects_game();
        insert(&mut game, pos![1, 1], "DUMMY", "WHITE");

        pick(&mut game, pos![1, 1]).unwrap();

        assert!(kept(&game).is_some(), "the turn always carries its ways");
        let moves = targets(&game);
        assert!(moves.is_empty(), "DUMMY should have no legal moves");
    }

    #[test]
    fn a_piece_picked_from_the_hand_keeps_the_turns_ways() {
        let mut game = load_shogi();
        game.state.pieces.clear();

        let current_player = game.current_player();
        game.state
            .hand
            .entry(current_player.clone())
            .or_default()
            .insert("PAWN".to_string(), 1);

        pick_hand(&mut game, "PAWN").unwrap();

        assert_eq!(chosen(&game), vec![Answer::Hand("PAWN".to_string())]);
        assert!(
            kept(&game).is_some(),
            "a piece picked from the hand carries the turn's ways"
        );
    }

    /// An answer that leads nowhere is refused and changes nothing: the piece stays picked.
    /// (The old flow put the piece back; the protocol leaves the answers as they were.)
    #[test]
    fn an_invalid_destination_is_refused_and_changes_nothing() {
        let mut game = load_side_effects_game();
        insert(&mut game, pos![0, 0], "CASTLER", "WHITE");

        pick(&mut game, pos![0, 0]).unwrap();
        assert!(!targets(&game).is_empty());
        let before = game.save_state().unwrap();
        let choice = game.choice();

        let result = go(&mut game, pos![9, 9]);
        assert!(result.is_err(), "expected invalid move error");

        assert_eq!(game.save_state().unwrap(), before, "the state is unchanged");
        assert_eq!(game.choice(), choice, "the choice is unchanged");
        assert!(ways_are_fresh(&game));
    }

    #[test]
    fn a_played_move_leaves_only_the_next_players_ways() {
        let mut game = load_chess();

        pick(&mut game, pos![4, 1]).unwrap();
        assert_eq!(chosen(&game), vec![Answer::Place(pos![4, 1])]);
        assert!(!targets(&game).is_empty());

        go(&mut game, pos![4, 3]).unwrap();

        assert!(
            phase_is_choosing(&game),
            "a played move leaves nothing chosen"
        );
        assert!(
            ways_are_fresh(&game),
            "the ways kept after a move are the next player's"
        );
    }

    #[test]
    fn a_promotion_keeps_only_its_ways_then_the_next_players() {
        let mut game = load_chess();
        game.state.pieces.clear();
        insert(&mut game, pos![0, 6], "PAWN", "WHITE");
        insert(&mut game, pos![4, 0], "KING", "WHITE");
        insert(&mut game, pos![7, 7], "KING", "BLACK");

        pick(&mut game, pos![0, 6]).unwrap();
        go(&mut game, pos![0, 7]).unwrap();

        assert!(
            !promotion_options(&game).is_empty(),
            "promotion move should enter transforming phase"
        );
        assert!(
            targets(&game).is_empty(),
            "transforming phase should offer no other move"
        );
        assert!(
            remaining(&game).iter().all(|way| way
                .answers
                .starts_with(&[Answer::Place(pos![0, 6]), Answer::Place(pos![0, 7])])),
            "only the promoting move's ways are left"
        );

        promote(&mut game, "QUEEN").unwrap();

        assert!(
            promotion_options(&game).is_empty(),
            "Transform should exit transforming phase"
        );
        assert!(
            phase_is_choosing(&game) && ways_are_fresh(&game),
            "the ways kept after the promotion are the next player's"
        );
    }

    #[test]
    fn a_played_drop_leaves_only_the_next_players_ways() {
        let mut game = load_shogi();
        let current_player = game.current_player();
        game.state
            .hand
            .entry(current_player)
            .or_default()
            .insert("ROOK".to_string(), 1);

        pick_hand(&mut game, "ROOK").unwrap();
        assert_eq!(chosen(&game), vec![Answer::Hand("ROOK".to_string())]);
        let drop_target = drop_squares(&game, "ROOK")
            .first()
            .cloned()
            .expect("expected at least one legal rook drop square");

        go(&mut game, drop_target).unwrap();

        assert!(
            phase_is_choosing(&game),
            "a played drop leaves nothing chosen"
        );
        assert!(
            ways_are_fresh(&game),
            "the ways kept after a drop are the next player's"
        );
    }
}
