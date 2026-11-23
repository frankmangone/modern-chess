//! Kalah (Mancala) without its free turn, played by the blocks and no rule of its own: pits and
//! stores are map places that hold many stones, a sowing is a `FOR_EACH` over a `FOLLOW` along a
//! one-way ring that skips the other player's store, and the score counts the stones kept in a
//! player's zone. The free turn after a last stone in the store needs turn stages (step 5), and
//! the capture of the opposite pit needs a link label of its own, so neither is here.
#[cfg(test)]
mod tests {
    use crate::logic::game::state::holder::Holder;
    use crate::logic::{Answer, Game, GamePhase, GameTransition};
    use crate::shared::{Position, Symbol};
    use crate::specs::parse_game_spec;

    fn load_game() -> Game {
        parse_game_spec("./src/tests/kalah/spec.json")
            .map(Game::from_spec)
            .expect("the kalah spec is read")
    }

    /// The stones in `place`.
    fn stones(game: &Game, place: &str) -> u32 {
        game.state
            .holders
            .get(&Position::new(place))
            .map_or(0, |held| held.count_of("STONE"))
    }

    /// Puts exactly `count` stones in `place`.
    fn set(game: &mut Game, place: &str, count: u32) {
        let at = Position::new(place);
        game.state.holders.remove(&at);
        if count > 0 {
            let mut holder = Holder::default();
            holder.add(Symbol::new(""), Symbol::new("STONE"), count);
            game.state.holders.insert(at, holder);
        }
        game.work_out_ways();
    }

    /// Sows the stones of the pit `place`.
    fn sow(game: &mut Game, place: &str) {
        game.transition(GameTransition::Choose(Answer::Place(Position::new(place))))
            .unwrap_or_else(|error| panic!("sowing {place} is refused: {error:?}"));
    }

    fn all_stones(game: &Game) -> u32 {
        game.state
            .holders
            .values()
            .map(|held| held.count_of("STONE"))
            .sum()
    }

    #[test]
    fn the_game_starts_with_four_stones_in_every_pit() {
        let game = load_game();
        for pit in ["a1", "a3", "a6", "b1", "b4", "b6"] {
            assert_eq!(stones(&game, pit), 4, "{pit}");
        }
        assert_eq!(stones(&game, "sa"), 0);
        assert_eq!(stones(&game, "sb"), 0);
        assert_eq!(all_stones(&game), 48);
        let ways = game.state.ways.as_ref().expect("ways");
        assert_eq!(ways.len(), 6, "SOUTH may sow any of its six pits");
    }

    #[test]
    fn a_sowing_drops_one_stone_in_each_pit_that_follows() {
        let mut game = load_game();
        sow(&mut game, "a1");
        assert_eq!(stones(&game, "a1"), 0);
        for pit in ["a2", "a3", "a4", "a5"] {
            assert_eq!(stones(&game, pit), 5, "{pit}");
        }
        assert_eq!(stones(&game, "a6"), 4);
        assert_eq!(game.current_player(), "NORTH");
        assert_eq!(all_stones(&game), 48);
    }

    #[test]
    fn a_sowing_that_passes_the_store_drops_a_stone_in_it() {
        let mut game = load_game();
        sow(&mut game, "a4");
        assert_eq!(stones(&game, "a5"), 5);
        assert_eq!(stones(&game, "a6"), 5);
        assert_eq!(stones(&game, "sa"), 1);
        assert_eq!(stones(&game, "b1"), 5);
        assert_eq!(stones(&game, "a4"), 0);
    }

    #[test]
    fn a_sowing_skips_the_other_players_store_and_goes_round() {
        let mut game = load_game();
        sow(&mut game, "a1");
        set(&mut game, "b1", 14);
        sow(&mut game, "b1");
        // b2..b6, its own store, a1..a6, the store of SOUTH skipped, then b1 and b2: 14.
        assert_eq!(stones(&game, "sa"), 0, "SOUTH's store was skipped");
        assert_eq!(stones(&game, "sb"), 1);
        assert_eq!(
            stones(&game, "b1"),
            1,
            "the lap came back to the pit it left"
        );
        assert_eq!(stones(&game, "b2"), 4 + 2);
        assert_eq!(stones(&game, "a6"), 4 + 1);
    }

    #[test]
    fn a_pit_with_no_stones_is_not_offered() {
        let mut game = load_game();
        set(&mut game, "a2", 0);
        let ways = game.state.ways.as_ref().expect("ways");
        assert_eq!(ways.len(), 5);
        assert!(game
            .transition(GameTransition::Choose(Answer::Place(Position::new("a2"))))
            .is_err());
        let wrong_side =
            game.transition(GameTransition::Choose(Answer::Place(Position::new("b1"))));
        assert!(
            wrong_side.is_err(),
            "a pit of the other player is not offered"
        );
    }

    #[test]
    fn the_game_ends_when_the_player_to_move_has_no_stones_and_the_stones_kept_decide() {
        let mut game = load_game();
        for pit in ["a1", "a2", "a3", "a4", "a5", "b2", "b3", "b4", "b5", "b6"] {
            set(&mut game, pit, 0);
        }
        set(&mut game, "a6", 1);
        set(&mut game, "b1", 0);
        set(&mut game, "sa", 5);
        set(&mut game, "sb", 20);
        // SOUTH sows its last stone into its store; NORTH then has nothing to sow.
        sow(&mut game, "a6");
        assert_eq!(stones(&game, "sa"), 6);
        assert_eq!(
            game.state.phase,
            GamePhase::GameOver {
                winner: Some("NORTH".to_string())
            }
        );
        let scores = game.scores().expect("a score");
        assert_eq!(scores.get("SOUTH"), Some(&6.0));
        assert_eq!(scores.get("NORTH"), Some(&20.0));
    }

    #[test]
    fn the_stones_left_on_a_side_count_for_that_side() {
        let mut game = load_game();
        for pit in [
            "a1", "a2", "a3", "a4", "a5", "a6", "b2", "b3", "b4", "b5", "b6",
        ] {
            set(&mut game, pit, 0);
        }
        set(&mut game, "b1", 3);
        set(&mut game, "sa", 10);
        set(&mut game, "sb", 20);
        let scores = game.scores().expect("a score");
        assert_eq!(scores.get("SOUTH"), Some(&10.0));
        assert_eq!(scores.get("NORTH"), Some(&23.0), "its pit and its store");
    }
}
