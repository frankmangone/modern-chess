//! Modern Chess: Capitalists against Communists, with conversions, a dispersal, layoffs and
//! elections. Each test sets a small position up by hand and plays one rule.
#[cfg(test)]
mod tests {
    use crate::logic::{Answer, Game, GameTransition, Piece};
    use crate::shared::Position;
    use crate::specs::parse_game_spec;
    use crate::tests::support::{go, in_check, moves_from, pick, targets};

    const CAP: &str = "CAPITALISTS";
    const COM: &str = "COMMUNISTS";

    fn load() -> Game {
        parse_game_spec("./specs/modern_chess.json")
            .map(Game::from_spec)
            .expect("the Modern Chess spec loads")
    }

    /// The opening position after the coin, with `player` to open.
    fn opened_by(player: &str) -> Game {
        (0..)
            .map(|seed| {
                let mut game = load();
                game.set_seed(seed);
                name(&mut game, "FLIP");
                game
            })
            .find(|game| game.current_player() == player)
            .expect("the coin lands on both sides")
    }

    /// An empty board with both leaders far from the action, `player` to move.
    fn empty(player: &str) -> Game {
        let mut game = opened_by(player);
        game.state.pieces.clear();
        put(&mut game, pos![7, 0], "ECONOMY", CAP);
        put(&mut game, pos![7, 7], "HEAD_OF_STATE", COM);
        game.state.current_turn = if player == CAP { 0 } else { 1 };
        game
    }

    fn put(game: &mut Game, at: Position, code: &str, player: &str) {
        game.state
            .pieces
            .insert(at, Piece::new(code.to_string(), player.to_string()));
    }

    fn at(game: &Game, place: Position) -> Option<(String, String)> {
        game.state
            .pieces
            .get(&place)
            .map(|piece| (piece.code.to_string(), piece.player.to_string()))
    }

    fn piece(code: &str, player: &str) -> Option<(String, String)> {
        Some((code.to_string(), player.to_string()))
    }

    fn name(game: &mut Game, action: &str) {
        game.transition(GameTransition::Choose(Answer::Action(action.to_string())))
            .expect("the action is offered");
    }

    #[test]
    fn the_capitalists_hold_the_first_turn_and_can_only_flip_the_coin() {
        let mut game = load();
        assert_eq!(game.current_player(), CAP);
        assert!(moves_from(&mut game, pos![0, 1]).is_empty(), "nothing moves before the coin");
        name(&mut game, "FLIP");
    }

    #[test]
    fn the_dollar_lets_the_capitalists_open_and_the_star_hands_the_opening_over() {
        let mut capitalists = opened_by(CAP);
        go_to(&mut capitalists, pos![0, 1], pos![0, 2]);
        assert_eq!(capitalists.current_player(), COM);

        let mut communists = opened_by(COM);
        go_to(&mut communists, pos![0, 6], pos![0, 5]);
        assert_eq!(communists.current_player(), CAP);
    }

    #[test]
    fn the_coin_is_flipped_once() {
        let mut game = opened_by(CAP);
        go_to(&mut game, pos![0, 1], pos![0, 2]);
        go_to(&mut game, pos![0, 6], pos![0, 5]);
        assert!(game
            .transition(GameTransition::Choose(Answer::Action("FLIP".to_string())))
            .is_err());
    }

    #[test]
    fn capitalists_open_and_every_people_starts_in_a_crowd() {
        let mut game = opened_by(CAP);
        // A People of the front row has neighbours on both sides and behind: it moves like a
        // king, so after a Capitalist move it can step diagonally forward onto an empty square.
        go_to(&mut game, pos![0, 1], pos![0, 2]);
        assert!(moves_from(&mut game, pos![3, 6]).contains(&pos![2, 5]));
    }

    fn go_to(game: &mut Game, from: Position, to: Position) {
        pick(game, from).expect("the piece can be picked");
        go(game, to).expect("the move is legal");
    }

    #[test]
    fn a_lone_people_steps_forward_only() {
        let mut game = empty(COM);
        put(&mut game, pos![3, 4], "PEOPLE", COM);
        assert_eq!(moves_from(&mut game, pos![3, 4]), vec![pos![3, 3]]);
    }

    #[test]
    fn a_lone_people_takes_diagonally_forward_and_straight_ahead() {
        let mut game = empty(COM);
        put(&mut game, pos![3, 4], "PEOPLE", COM);
        put(&mut game, pos![2, 3], "WORKER", CAP);
        put(&mut game, pos![3, 3], "WORKER", CAP);
        put(&mut game, pos![2, 5], "WORKER", CAP);
        assert_eq!(moves_from(&mut game, pos![3, 4]), vec![pos![2, 3], pos![3, 3]]);
    }

    #[test]
    fn a_people_with_one_neighbour_also_steps_sideways_and_back() {
        let mut game = empty(COM);
        put(&mut game, pos![3, 4], "PEOPLE", COM);
        put(&mut game, pos![4, 4], "PEOPLE", COM);
        assert_eq!(
            moves_from(&mut game, pos![3, 4]),
            vec![pos![2, 4], pos![3, 3], pos![3, 5]]
        );
    }

    #[test]
    fn a_people_with_two_neighbours_moves_like_a_king() {
        let mut game = empty(COM);
        put(&mut game, pos![3, 4], "PEOPLE", COM);
        put(&mut game, pos![2, 4], "PEOPLE", COM);
        put(&mut game, pos![4, 4], "PEOPLE", COM);
        assert_eq!(
            moves_from(&mut game, pos![3, 4]),
            vec![pos![2, 3], pos![2, 5], pos![3, 3], pos![3, 5], pos![4, 3], pos![4, 5]]
        );
    }

    #[test]
    fn a_people_on_the_far_rank_becomes_a_bureaucrat() {
        let mut game = empty(COM);
        put(&mut game, pos![2, 1], "PEOPLE", COM);
        go_to(&mut game, pos![2, 1], pos![2, 0]);
        assert_eq!(at(&game, pos![2, 0]), piece("BUREAUCRAT", COM));
    }

    #[test]
    fn a_worker_on_the_far_rank_becomes_a_burguess() {
        let mut game = empty(CAP);
        put(&mut game, pos![2, 6], "WORKER", CAP);
        go_to(&mut game, pos![2, 6], pos![2, 7]);
        assert_eq!(at(&game, pos![2, 7]), piece("BURGUESS", CAP));
    }

    #[test]
    fn a_bureaucrat_alone_leaps_and_slides_next_to_a_thinker() {
        let mut game = empty(COM);
        put(&mut game, pos![3, 3], "BUREAUCRAT", COM);
        assert_eq!(moves_from(&mut game, pos![3, 3]).len(), 8);

        let mut game = empty(COM);
        put(&mut game, pos![3, 3], "BUREAUCRAT", COM);
        put(&mut game, pos![2, 2], "THINKER", COM);
        let moves = moves_from(&mut game, pos![3, 3]);
        assert!(moves.contains(&pos![3, 7]), "slides up the file: {moves:?}");
        assert!(moves.contains(&pos![0, 3]), "slides along the rank: {moves:?}");
        assert!(!moves.contains(&pos![6, 6]), "no bishop slide without the Head of State");
    }

    #[test]
    fn a_television_turns_an_adjacent_people_into_a_worker_and_stays() {
        let mut game = empty(CAP);
        put(&mut game, pos![3, 3], "TELEVISION", CAP);
        put(&mut game, pos![3, 4], "PEOPLE", COM);
        go_to(&mut game, pos![3, 3], pos![3, 4]);
        assert_eq!(at(&game, pos![3, 4]), piece("WORKER", CAP));
        assert_eq!(at(&game, pos![3, 3]), piece("TELEVISION", CAP));
    }

    #[test]
    fn a_people_converted_on_the_far_rank_becomes_a_burguess() {
        let mut game = empty(CAP);
        put(&mut game, pos![3, 6], "TELEVISION", CAP);
        put(&mut game, pos![3, 7], "PEOPLE", COM);
        go_to(&mut game, pos![3, 6], pos![3, 7]);
        assert_eq!(at(&game, pos![3, 7]), piece("BURGUESS", CAP));
    }

    #[test]
    fn a_television_cannot_take() {
        let mut game = empty(CAP);
        put(&mut game, pos![3, 3], "TELEVISION", CAP);
        put(&mut game, pos![3, 4], "BUREAUCRAT", COM);
        assert!(!moves_from(&mut game, pos![3, 3]).contains(&pos![3, 4]));
    }

    #[test]
    fn a_thinker_turns_an_adjacent_worker_into_people() {
        let mut game = empty(COM);
        put(&mut game, pos![3, 3], "THINKER", COM);
        put(&mut game, pos![3, 2], "WORKER", CAP);
        go_to(&mut game, pos![3, 3], pos![3, 2]);
        assert_eq!(at(&game, pos![3, 2]), piece("PEOPLE", COM));
    }

    #[test]
    fn the_police_disperses_the_people_around_it() {
        let mut game = empty(CAP);
        put(&mut game, pos![3, 0], "POLICE", CAP);
        put(&mut game, pos![2, 5], "PEOPLE", COM);
        put(&mut game, pos![3, 5], "PEOPLE", COM);
        put(&mut game, pos![2, 4], "PEOPLE", COM);
        put(&mut game, pos![4, 4], "PEOPLE", COM);
        put(&mut game, pos![3, 6], "BUREAUCRAT", COM); // blocks the push of [3, 5]

        pick(&mut game, pos![3, 0]).unwrap();
        go(&mut game, pos![3, 4]).unwrap();
        name(&mut game, "DISPERSE");

        assert_eq!(at(&game, pos![3, 4]), None, "the Police is sacrificed");
        assert_eq!(at(&game, pos![1, 6]), piece("PEOPLE", COM));
        assert_eq!(at(&game, pos![1, 4]), piece("PEOPLE", COM));
        assert_eq!(at(&game, pos![5, 4]), piece("PEOPLE", COM));
        assert_eq!(at(&game, pos![3, 5]), piece("PEOPLE", COM), "blocked, it stays");
        assert_eq!(game.current_player(), COM);
    }

    #[test]
    fn the_police_only_disperses_next_to_people() {
        let mut game = empty(CAP);
        put(&mut game, pos![3, 0], "POLICE", CAP);
        pick(&mut game, pos![3, 0]).unwrap();
        go(&mut game, pos![3, 4]).unwrap();
        assert_eq!(at(&game, pos![3, 4]), piece("POLICE", CAP), "a plain move, played at once");
    }

    /// A Multinational on [0, 2] with a Burguess out of the way and one Worker.
    fn multinational_with_one_worker(player: &str) -> Game {
        let mut game = empty(player);
        put(&mut game, pos![0, 2], "MULTINATIONAL", CAP);
        put(&mut game, pos![7, 3], "BURGUESS", CAP);
        put(&mut game, pos![6, 1], "WORKER", CAP);
        game
    }

    #[test]
    fn the_multinational_moves_as_far_as_there_are_workers() {
        let mut game = multinational_with_one_worker(CAP);
        let moves = moves_from(&mut game, pos![0, 2]);
        assert_eq!(moves.len(), 5, "one square around: {moves:?}");

        put(&mut game, pos![6, 2], "WORKER", CAP);
        put(&mut game, pos![6, 3], "WORKER", CAP);
        let moves = moves_from(&mut game, pos![0, 2]);
        assert!(moves.contains(&pos![0, 5]) && !moves.contains(&pos![0, 6]), "{moves:?}");
    }

    #[test]
    fn without_a_burguess_the_multinational_is_a_king() {
        let mut game = multinational_with_one_worker(CAP);
        game.state.pieces.remove(&pos![7, 3]);
        put(&mut game, pos![6, 2], "WORKER", CAP);
        put(&mut game, pos![6, 3], "WORKER", CAP);
        assert_eq!(moves_from(&mut game, pos![0, 2]).len(), 5);
    }

    #[test]
    fn a_layoff_gives_the_multinational_its_full_range_for_one_move() {
        let mut game = multinational_with_one_worker(CAP);
        pick(&mut game, pos![0, 2]).unwrap();
        go(&mut game, pos![6, 1]).unwrap(); // the Worker laid off
        assert!(targets(&game).contains(&pos![0, 7]));
        go(&mut game, pos![0, 6]).unwrap();

        assert_eq!(at(&game, pos![0, 6]), piece("MULTINATIONAL", CAP));
        assert_eq!(at(&game, pos![6, 1]), None, "the Worker is gone");
        let flags = &game.state.pieces[&pos![0, 6]].state;
        assert!(flags.is_empty(), "the boost lasts one move: {flags:?}");
    }

    #[test]
    fn the_multinational_gives_check_at_its_boosted_range() {
        let mut game = multinational_with_one_worker(COM);
        put(&mut game, pos![0, 7], "PEOPLE", COM);
        game.state.pieces.remove(&pos![7, 7]);
        put(&mut game, pos![0, 6], "HEAD_OF_STATE", COM);
        assert!(in_check(&game, &["HEAD_OF_STATE"]), "a layoff could reach it");

        game.state.pieces.remove(&pos![6, 1]);
        assert!(!in_check(&game, &["HEAD_OF_STATE"]), "no Worker to lay off");
    }

    #[test]
    fn elections_pass_the_head_of_state_to_a_bureaucrat_once() {
        let mut game = empty(COM);
        game.state.pieces.remove(&pos![7, 7]);
        put(&mut game, pos![4, 7], "HEAD_OF_STATE", COM);
        put(&mut game, pos![0, 7], "BUREAUCRAT", COM);
        put(&mut game, pos![4, 0], "POLICE", CAP);
        assert!(in_check(&game, &["HEAD_OF_STATE"]));

        pick(&mut game, pos![4, 7]).unwrap();
        go(&mut game, pos![0, 7]).unwrap();

        assert_eq!(at(&game, pos![0, 7]), piece("HEAD_OF_STATE", COM));
        assert_eq!(at(&game, pos![4, 7]), piece("BUREAUCRAT", COM));
        assert_eq!(game.counter(COM, "ELECTIONS"), 1);

        // Once per game: with another Bureaucrat at hand, the Head of State cannot be passed on.
        go_to(&mut game, pos![7, 0], pos![7, 1]);
        put(&mut game, pos![1, 5], "BUREAUCRAT", COM);
        pick(&mut game, pos![0, 7]).unwrap();
        assert!(go(&mut game, pos![1, 5]).is_err());
    }

    #[test]
    fn an_election_cannot_put_the_head_of_state_in_check() {
        let mut game = empty(COM);
        put(&mut game, pos![0, 7], "BUREAUCRAT", COM);
        put(&mut game, pos![0, 0], "POLICE", CAP);
        pick(&mut game, pos![7, 7]).unwrap();
        assert!(go(&mut game, pos![0, 7]).is_err());
    }
}
