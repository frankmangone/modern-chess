//! Snakes and Ladders, played by the blocks and no rule of its own: the squares are map places that
//! hold two tokens, a turn rolls a die into a tray and walks the mover's token along the `step`
//! links, and a snake or a ladder is a one-way link under the label `jump` that the walk follows
//! once when it ends. A roll that would pass the last square moves nothing (the finish needs the
//! exact number).
#[cfg(test)]
mod tests {
    use crate::logic::places::holders::{bag_position, Holder};
    use crate::logic::{Answer, Game, GamePhase, GameTransition};
    use crate::shared::{Position, Symbol};
    use crate::specs::parse_game_spec;

    const LADDERS: [(u32, u32); 9] = [
        (1, 38),
        (4, 14),
        (9, 31),
        (21, 42),
        (28, 84),
        (36, 44),
        (51, 67),
        (71, 91),
        (80, 100),
    ];
    const SNAKES: [(u32, u32); 10] = [
        (16, 6),
        (47, 26),
        (49, 11),
        (56, 53),
        (62, 19),
        (64, 60),
        (87, 24),
        (93, 73),
        (95, 75),
        (98, 78),
    ];

    fn load() -> Game {
        parse_game_spec("./specs/snakes_and_ladders.json")
            .map(Game::from_spec)
            .expect("the spec reads")
    }

    /// The square `token` stands on.
    fn square(game: &Game, token: &str) -> u32 {
        let mut found = game
            .state
            .holders
            .iter()
            .filter(|(_, held)| held.count_of(token) > 0)
            .map(|(place, _)| place.as_str().parse::<u32>().expect("a square"));
        let square = found.next().expect("the token is on the board");
        assert!(found.next().is_none(), "one token of {token}");
        square
    }

    /// Puts `token` on `place`, and nowhere else.
    fn put(game: &mut Game, token: &str, place: u32) {
        for held in game.state.holders.values_mut() {
            if held.count_of(token) > 0 {
                held.take(token, 1);
            }
        }
        let at = Position::new(&place.to_string());
        game.state
            .holders
            .entry(at)
            .or_insert_with(Holder::default)
            .add(Symbol::new(""), Symbol::new(token), 1);
        game.work_out_ways();
    }

    fn rolled(game: &Game) -> u32 {
        game.state
            .holders
            .get(&bag_position("TRAY", None))
            .map_or(0, |held| held.count_of("PIP"))
    }

    fn roll(game: &mut Game) {
        game.transition(GameTransition::Choose(Answer::Action("ROLL".into())))
            .unwrap_or_else(|error| panic!("rolling is refused: {error:?}"));
    }

    /// A game with RED to move, its token on `from` and BLUE's on `other`, whose first roll is `pips`
    /// (found by trying seeds: the roll is a function of the seed).
    fn with_roll(from: u32, other: u32, pips: u32) -> Game {
        for seed in 0..200 {
            let mut game = load();
            game.set_seed(seed);
            put(&mut game, "RED_TOKEN", from);
            put(&mut game, "BLUE_TOKEN", other);
            let mut trial = game.clone();
            roll(&mut trial);
            if rolled(&trial) == pips {
                return game;
            }
        }
        panic!("no seed rolls {pips}");
    }

    #[test]
    fn both_tokens_start_off_the_board_and_red_rolls_first() {
        let game = load();
        assert_eq!(square(&game, "RED_TOKEN"), 0);
        assert_eq!(square(&game, "BLUE_TOKEN"), 0);
        assert_eq!(game.current_player(), "RED");
        assert_eq!(
            game.state.ways.as_ref().map(Vec::len),
            Some(1),
            "one way: roll"
        );
    }

    #[test]
    fn a_turn_walks_the_movers_token_by_the_roll_and_hands_on() {
        for pips in 1..=6 {
            let mut game = with_roll(20, 50, pips);
            roll(&mut game);
            let landed = 20 + pips;
            let expected = SNAKES
                .iter()
                .chain(LADDERS.iter())
                .find(|(from, _)| *from == landed)
                .map_or(landed, |(_, to)| *to);
            assert_eq!(square(&game, "RED_TOKEN"), expected, "rolled {pips}");
            assert_eq!(square(&game, "BLUE_TOKEN"), 50, "the other token stays");
            assert_eq!(game.current_player(), "BLUE");
        }
    }

    #[test]
    fn a_ladder_carries_the_token_up() {
        let mut game = with_roll(3, 10, 1);
        roll(&mut game);
        assert_eq!(
            square(&game, "RED_TOKEN"),
            14,
            "square 4 is the foot of a ladder"
        );
    }

    #[test]
    fn a_snake_carries_the_token_down() {
        let mut game = with_roll(15, 10, 1);
        roll(&mut game);
        assert_eq!(square(&game, "RED_TOKEN"), 6, "square 16 is a snake's head");
    }

    #[test]
    fn a_jump_is_taken_once_only() {
        // 1 → 38 is a ladder; 38 is not the foot of another.
        let mut game = with_roll(0, 10, 1);
        roll(&mut game);
        assert_eq!(square(&game, "RED_TOKEN"), 38);
    }

    #[test]
    fn blue_walks_blues_token_and_the_two_may_share_a_square() {
        let mut game = with_roll(10, 10, 3);
        roll(&mut game);
        assert_eq!(square(&game, "RED_TOKEN"), 13);
        // Blue to move, on the same square red left; give it a roll that lands on red's square.
        let mut blue = with_roll(13, 10, 3);
        blue.state.current_turn = 1;
        blue.work_out_ways();
        roll(&mut blue);
        assert_eq!(square(&blue, "BLUE_TOKEN"), 13, "both tokens on 13");
        assert_eq!(square(&blue, "RED_TOKEN"), 13);
    }

    #[test]
    fn the_finish_needs_the_exact_roll() {
        // From 97, a 3 lands on 100 and wins; a 4 would pass it and moves nothing.
        let mut over = with_roll(97, 10, 4);
        roll(&mut over);
        assert_eq!(square(&over, "RED_TOKEN"), 97, "an overshoot stays put");
        assert!(matches!(over.state.phase, GamePhase::Choosing { .. }));
        assert_eq!(over.current_player(), "BLUE", "the turn still passes");

        let mut exact = with_roll(97, 10, 3);
        roll(&mut exact);
        assert_eq!(square(&exact, "RED_TOKEN"), 100);
        assert!(matches!(exact.state.phase, GamePhase::GameOver { .. }));
    }

    #[test]
    fn a_ladder_to_the_last_square_wins() {
        let mut game = with_roll(79, 10, 1);
        roll(&mut game);
        assert_eq!(
            square(&game, "RED_TOKEN"),
            100,
            "80 is the foot of a ladder to 100"
        );
        assert!(matches!(game.state.phase, GamePhase::GameOver { .. }));
    }

    #[test]
    fn games_end_with_one_winner() {
        for seed in 0..40 {
            let mut game = load();
            game.set_seed(seed);
            let mut turns = 0;
            while matches!(game.state.phase, GamePhase::Choosing { .. }) {
                roll(&mut game);
                turns += 1;
                assert!(turns < 4000, "seed {seed} does not end");
            }
            let (red, blue) = (square(&game, "RED_TOKEN"), square(&game, "BLUE_TOKEN"));
            assert!(
                (red == 100) != (blue == 100),
                "exactly one token on 100 at seed {seed}: {red} {blue}"
            );
        }
    }
}
