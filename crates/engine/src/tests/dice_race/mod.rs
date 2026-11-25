//! A dice race, the proof game of step 6, written only with blocks: each player has a runner on a
//! lane of 18 places; a turn rolls a die into the shared tray and runs the runner that many places;
//! the first runner on one of the last six places wins. It needs a roll (`ROLL`), a count of pips
//! read as a number (`FOLLOW` with a counted `steps`), and a seed that makes it replay exactly.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::places::holders::bag_position;
    use crate::logic::{Answer, Game, GamePhase, GameTransition};
    use crate::shared::Position;
    use crate::specs::parse_game_spec_str;

    const LANE: usize = 18;
    const FINISH_FROM: usize = 12;

    fn lane(prefix: &str) -> Vec<String> {
        (0..LANE).map(|place| format!("{prefix}{place}")).collect()
    }

    fn spec() -> Value {
        let mut places = Vec::new();
        let mut links = Vec::new();
        for (row, prefix) in [(0, "s"), (1, "n")] {
            let names = lane(prefix);
            for (column, name) in names.iter().enumerate() {
                places.push(json!({ "name": name, "at": [column, row] }));
                if let Some(next) = names.get(column + 1) {
                    links.push(json!({ "from": name, "to": next }));
                }
            }
        }
        let finish = |prefix: &str| lane(prefix).split_off(FINISH_FROM);
        json!({
            "format_version": 1,
            "name": "DICE RACE",
            "board": { "shape": "map", "places": places, "links": links },
            "players": [
                { "name": "SOUTH", "starting_positions": [{ "piece": "RUNNER", "positions": ["s0"] }] },
                { "name": "NORTH", "starting_positions": [{ "piece": "RUNNER", "positions": ["n0"] }] }
            ],
            "turns": { "order": ["SOUTH", "NORTH"] },
            "pieces": [
                { "code": "RUNNER", "name": "runner", "moves": [] },
                { "code": "PIP", "name": "pip", "moves": [] }
            ],
            "zones": [{ "name": "FINISH", "places": { "SOUTH": finish("s"), "NORTH": finish("n") } }],
            "bags": [{ "name": "TRAY" }],
            "actions": [{ "name": "RUN", "do": [
                { "choose": "PIECE", "where": { "owner": "ALLY" } },
                { "effect": "REMOVE", "at": { "bag": "TRAY" }, "count": "ALL" },
                { "effect": "ROLL", "at": { "bag": "TRAY" },
                  "faces": [{ "PIP": 1 }, { "PIP": 2 }, { "PIP": 3 }, { "PIP": 4 }, { "PIP": 5 }, { "PIP": 6 }] },
                { "effect": "REMOVE", "at": { "place": "START" } },
                { "effect": "FOR_EACH",
                  "in": { "region": "FOLLOW", "from": { "place": "START" },
                          "steps": { "count": { "in": { "region": "AT", "at": { "bag": "TRAY" } },
                                                 "where": { "codes": ["PIP"] } } } },
                  "do": [
                      { "effect": "REMOVE", "at": { "place": "LAST" } },
                      { "effect": "PLACE", "at": { "place": "EACH" }, "code": "RUNNER" }
                  ] }
            ] }],
            "end_checks": [{
                "when": { "test": "SOME", "in": { "region": "BOARD" }, "that": { "test": "ALL", "of": [
                    { "test": "IN_ZONE", "at": { "place": "EACH" }, "zone": "FINISH" },
                    { "test": "COUNT", "in": { "region": "AT", "at": { "place": "EACH" } },
                      "where": { "owner": "ALLY", "codes": ["RUNNER"] } }
                ] } },
                "then": { "outcome": "WIN" }
            }]
        })
    }

    fn new_game(seed: u64) -> Game {
        let spec = parse_game_spec_str(&spec().to_string()).expect("the dice race reads");
        let mut game = Game::from_spec(spec);
        game.set_seed(seed);
        game
    }

    /// Where `player`'s runner stands, as its index on the lane.
    fn runner(game: &Game, player: &str) -> usize {
        let prefix = if player == "SOUTH" { 's' } else { 'n' };
        game.state
            .pieces
            .iter()
            .find(|(_, piece)| piece.player == player)
            .and_then(|(place, _)| place.as_str().strip_prefix(prefix)?.parse().ok())
            .expect("the runner is on its lane")
    }

    fn pips(game: &Game) -> u32 {
        game.state
            .holders
            .get(&bag_position("TRAY", None))
            .map_or(0, |held| held.count_of("PIP"))
    }

    /// Plays a turn: picks the mover's runner, which rolls and runs.
    fn run(game: &mut Game) {
        let at = game
            .state
            .pieces
            .iter()
            .find(|(_, piece)| piece.player == game.current_player())
            .map(|(place, _)| *place)
            .expect("the mover has a runner");
        game.transition(GameTransition::Choose(Answer::Place(at)))
            .unwrap_or_else(|error| panic!("running is refused: {error:?}"));
    }

    fn play_to_the_end(game: &mut Game) -> usize {
        let mut turns = 0;
        while matches!(game.state.phase, GamePhase::Choosing { .. }) {
            assert!(turns < 200, "the race never ended");
            run(game);
            turns += 1;
        }
        turns
    }

    #[test]
    fn the_runners_wait_at_the_start_with_one_way_to_play() {
        let game = new_game(1);
        assert!(game.rules.chance);
        assert_eq!(runner(&game, "SOUTH"), 0);
        assert_eq!(runner(&game, "NORTH"), 0);
        assert_eq!(pips(&game), 0);
        assert_eq!(game.state.ways.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn a_turn_rolls_a_die_and_runs_that_many_places() {
        let mut game = new_game(1);
        run(&mut game);
        let rolled = pips(&game);
        assert!((1..=6).contains(&rolled), "{rolled}");
        assert_eq!(runner(&game, "SOUTH"), rolled as usize);
        assert_eq!(runner(&game, "NORTH"), 0);
        assert_eq!(game.current_player(), "NORTH");
        assert_eq!(game.state.draws, 1);
        let record = game.history().last().expect("recorded");
        assert_eq!(record.rolled.len(), 1);
        assert_eq!(record.rolled[0].face.get("PIP"), Some(&rolled));
        assert_eq!(game.state.pieces.len(), 2, "one runner each, none left behind");
    }

    #[test]
    fn the_last_roll_stays_in_the_tray_until_the_next_one() {
        let mut game = new_game(2);
        run(&mut game);
        let first = pips(&game);
        run(&mut game);
        let second = pips(&game);
        assert!((1..=6).contains(&first) && (1..=6).contains(&second));
        assert_eq!(
            game.history().iter().map(|turn| turn.rolled.len()).sum::<usize>(),
            2
        );
        assert_eq!(runner(&game, "NORTH"), second as usize);
    }

    #[test]
    fn a_race_ends_with_the_first_runner_on_the_finish_and_every_roll_recorded() {
        let mut game = new_game(3);
        let turns = play_to_the_end(&mut game);
        let GamePhase::GameOver { .. } = game.state.phase else {
            panic!("the race is over");
        };
        let mover = game.history().last().map(|turn| turn.player.clone()).expect("a last turn");
        assert!(runner(&game, &mover) >= FINISH_FROM);
        let other = if mover == "SOUTH" { "NORTH" } else { "SOUTH" };
        assert!(runner(&game, other) < FINISH_FROM, "the other runner has not finished");
        assert_eq!(game.history().len(), turns);
        assert!(game
            .history()
            .iter()
            .all(|turn| turn.rolled.len() == 1 && (1..=6).contains(&turn.rolled[0].face["PIP"])));
        // Both runners moved by exactly what the history says they rolled.
        for player in ["SOUTH", "NORTH"] {
            let total: u32 = game
                .history()
                .iter()
                .filter(|turn| turn.player == player)
                .map(|turn| turn.rolled[0].face["PIP"])
                .sum();
            assert_eq!(runner(&game, player), total as usize, "{player}");
        }
    }

    #[test]
    fn the_same_seed_replays_the_same_race_and_another_seed_does_not() {
        let race = |seed: u64| {
            let mut game = new_game(seed);
            play_to_the_end(&mut game);
            game.history()
                .iter()
                .map(|turn| turn.rolled[0].face["PIP"])
                .collect::<Vec<_>>()
        };
        assert_eq!(race(10), race(10));
        let races: std::collections::BTreeSet<Vec<u32>> = (0..20).map(race).collect();
        assert!(races.len() > 15, "20 seeds gave {} different races", races.len());
    }

    #[test]
    fn a_race_saved_and_resumed_goes_on_exactly_as_if_it_had_not_stopped() {
        let mut straight = new_game(31);
        let mut paused = new_game(31);
        for _ in 0..3 {
            run(&mut straight);
            run(&mut paused);
        }
        let saved = paused.save_state().expect("saves");
        let mut resumed = Game::from_spec(
            parse_game_spec_str(&spec().to_string()).expect("the dice race reads"),
        );
        resumed.restore_state(&saved).expect("loads");
        assert_eq!(resumed.state.draws, 3);
        play_to_the_end(&mut straight);
        play_to_the_end(&mut resumed);
        let rolls = |game: &Game| {
            game.history()
                .iter()
                .map(|turn| turn.rolled[0].face["PIP"])
                .collect::<Vec<_>>()
        };
        assert_eq!(rolls(&straight), rolls(&resumed));
        assert_eq!(straight.state.draws, resumed.state.draws);
    }

    #[test]
    fn a_race_without_a_seed_plays_the_sequence_of_seed_zero() {
        let spec = parse_game_spec_str(&spec().to_string()).expect("the dice race reads");
        let mut unseeded = Game::from_spec(spec);
        let mut zero = new_game(0);
        for _ in 0..4 {
            run(&mut unseeded);
            run(&mut zero);
        }
        assert_eq!(unseeded.state.seed, None);
        assert_eq!(runner(&unseeded, "SOUTH"), runner(&zero, "SOUTH"));
        assert_eq!(runner(&unseeded, "NORTH"), runner(&zero, "NORTH"));
    }

    #[test]
    fn each_face_comes_up_about_a_sixth_of_the_time() {
        let mut seen = [0u32; 6];
        let seeds = 1200u64;
        for seed in 0..seeds {
            let mut game = new_game(seed.wrapping_mul(0x9E37_79B9));
            run(&mut game);
            if let Some(slot) = seen.get_mut(pips(&game) as usize - 1) {
                *slot += 1;
            }
        }
        // 200 expected; the standard deviation is about 13, so 6 of them is 80 either way.
        for (face, count) in seen.iter().enumerate() {
            assert!((120..=280).contains(count), "face {} came up {count} times: {seen:?}", face + 1);
        }
    }

    #[test]
    fn the_tray_and_the_runners_are_all_a_view_needs_to_show_the_roll() {
        let mut game = new_game(5);
        run(&mut game);
        let bags = game.bags();
        let tray = bags.iter().find(|bag| bag.name == "TRAY").expect("the tray is a bag");
        assert_eq!(tray.pieces.get("PIP"), Some(&pips(&game)));
        assert!(game.piece_at_position(&Position::new(format!("s{}", pips(&game)))).is_some());
    }
}
