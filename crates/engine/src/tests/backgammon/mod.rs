//! Backgammon, played by the blocks and no rule of its own: the points are map places that hold
//! fifteen checkers, a die is a piece in the mover's own `DICE` bag (the number rolled is the code,
//! `D1` to `D6`, and a double puts two more), each side's bar is a bag, and the trays are places off the
//! points. A turn is `ROLL`, which enters the stage `PLAY`; every move of the stage is chosen by
//! where the checker goes (a point, or the tray to bear off), and the die that moves it is the
//! action (`MOVE_3`, `ENTER_3`, `OFF_3`).
#[cfg(test)]
mod tests {
    use crate::logic::places::holders::bag_position;
    use crate::logic::{Answer, Game, GamePhase, GameTransition};
    use crate::shared::{Position, Symbol};
    use crate::specs::parse_game_spec;

    const W: &str = "WHITE_CHECKER";
    const B: &str = "BLACK_CHECKER";

    fn load() -> Game {
        parse_game_spec("./specs/backgammon.json")
            .map(Game::from_spec)
            .expect("the spec reads")
    }

    fn place(name: &str) -> Position {
        Position::new(name)
    }

    fn add(game: &mut Game, at: Position, owner: &str, code: &str, count: u32) {
        game.state
            .holders
            .entry(at)
            .or_default()
            .add(Symbol::new(owner), Symbol::new(code), count);
    }

    /// How many `code` stand on `point` (a point or a tray).
    fn on(game: &Game, point: &str, code: &str) -> u32 {
        game.state
            .holders
            .get(&place(point))
            .map_or(0, |held| held.count_of(code))
    }

    /// The name of `player`'s bar, a bag the two sides do not share.
    fn bar_name(player: &str) -> String {
        match player {
            "WHITE" => "White bar".to_string(),
            "BLACK" => "Black bar".to_string(),
            other => panic!("{other} is not a player"),
        }
    }

    fn bar(game: &Game, player: &str, code: &str) -> u32 {
        game.state
            .holders
            .get(&bag_position(&bar_name(player), None))
            .map_or(0, |held| held.count_of(code))
    }

    /// The dice `player` still holds, as the numbers, sorted.
    fn dice(game: &Game, player: &str) -> Vec<u32> {
        let mut found = Vec::new();
        if let Some(held) = game.state.holders.get(&bag_position("DICE", Some(player))) {
            for face in 1..=6 {
                for _ in 0..held.count_of(&format!("D{face}")) {
                    found.push(face);
                }
            }
        }
        found
    }

    /// An empty board with these checkers on it: `white` and `black` as (point, count).
    fn position(white: &[(&str, u32)], black: &[(&str, u32)], mover: u8) -> Game {
        let mut game = load();
        game.state.holders.clear();
        for (point, count) in white {
            add(&mut game, place(point), "", W, *count);
        }
        for (point, count) in black {
            add(&mut game, place(point), "", B, *count);
        }
        game.state.current_turn = mover;
        game.work_out_ways();
        game
    }

    fn put_on_bar(game: &mut Game, player: &str, code: &str, count: u32) {
        add(game, bag_position(&bar_name(player), None), "", code, count);
        game.work_out_ways();
    }

    fn act(game: &mut Game, name: &str) {
        game.transition(GameTransition::Choose(Answer::Action(name.into())))
            .unwrap_or_else(|error| panic!("{name} is refused: {error:?}"));
    }

    fn go(game: &mut Game, point: &str) {
        game.transition(GameTransition::Choose(Answer::Place(place(point))))
            .unwrap_or_else(|error| panic!("{point} is refused: {error:?}"));
    }

    fn refused(game: &Game, point: &str) -> bool {
        game.clone()
            .transition(GameTransition::Choose(Answer::Place(place(point))))
            .is_err()
    }

    /// The points the mover may answer now.
    fn offered(game: &Game) -> Vec<String> {
        let mut names: Vec<String> = game
            .choice()
            .places
            .iter()
            .map(|option| option.name.as_str().to_string())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// `game` after the mover's `ROLL`, with exactly the dice `faces` (found by trying seeds: the
    /// roll is a function of the seed).
    fn rolled(game: &Game, faces: [u32; 2]) -> Game {
        let mover = game.current_player();
        let mut want = faces.to_vec();
        want.sort_unstable();
        if faces[0] == faces[1] {
            want = vec![faces[0]; 4];
        }
        for seed in 0..3000 {
            let mut trial = game.clone();
            trial.set_seed(seed);
            act(&mut trial, "ROLL");
            // Looked at in the roller's bag: with nothing to play the turn is handed on at once.
            if dice(&trial, &mover) == want {
                return trial;
            }
        }
        panic!("no seed rolls {faces:?}");
    }

    fn over(game: &Game) -> bool {
        matches!(game.state.phase, GamePhase::GameOver { .. })
    }

    #[test]
    fn the_standard_start_with_white_to_roll() {
        let game = load();
        for (point, white, black) in [
            ("24", 2, 0),
            ("13", 5, 0),
            ("8", 3, 0),
            ("6", 5, 0),
            ("1", 0, 2),
            ("12", 0, 5),
            ("17", 0, 3),
            ("19", 0, 5),
        ] {
            assert_eq!(on(&game, point, W), white, "white on {point}");
            assert_eq!(on(&game, point, B), black, "black on {point}");
        }
        assert_eq!(game.current_player(), "WHITE");
        assert_eq!(game.state.ways.as_ref().map(Vec::len), Some(1), "one way: roll");
    }

    #[test]
    fn a_roll_of_two_dice_is_two_moves_and_the_dice_are_spent_one_by_one() {
        let game = rolled(&load(), [3, 1]);
        assert_eq!(dice(&game, "WHITE"), vec![1, 3]);
        assert_eq!(game.current_player(), "WHITE", "the turn goes on after the roll");
        // 24→21 and 24→23 with 3 and 1 are open; 6→5 and 8→7 and 8→5 and 13→12(black) is blocked.
        let offered = offered(&game);
        assert!(offered.contains(&"21".to_string()));
        assert!(offered.contains(&"5".to_string()));
        assert!(!offered.contains(&"12".to_string()), "black holds 12 with five");
        let mut game = game;
        go(&mut game, "21");
        assert_eq!(on(&game, "21", W), 1);
        assert_eq!(on(&game, "24", W), 1);
        assert_eq!(dice(&game, "WHITE"), vec![1], "the 3 was spent");
        assert_eq!(game.current_player(), "WHITE");
        go(&mut game, "23");
        assert_eq!(dice(&game, "WHITE"), vec![] as Vec<u32>);
        assert_eq!(game.current_player(), "BLACK", "both dice played: the turn passes");
    }

    #[test]
    fn a_double_is_four_moves() {
        let mut game = rolled(&load(), [2, 2]);
        assert_eq!(dice(&game, "WHITE"), vec![2, 2, 2, 2]);
        for _ in 0..3 {
            go(&mut game, "11");
            assert_eq!(game.current_player(), "WHITE");
        }
        go(&mut game, "11");
        assert_eq!(on(&game, "11", W), 4, "four checkers 13→11");
        assert_eq!(game.current_player(), "BLACK");
    }

    #[test]
    fn a_point_with_two_enemy_checkers_is_closed() {
        let game = rolled(&load(), [6, 5]);
        // 24→18 is open, 24→19 is black's five; 13→7 is open; 13→8 own; 6→1 black's two.
        assert!(refused(&game, "19"));
        assert!(refused(&game, "1"));
        assert!(offered(&game).contains(&"18".to_string()));
    }

    #[test]
    fn a_lone_enemy_checker_is_hit_and_goes_to_the_bar() {
        let game = position(&[("8", 2)], &[("5", 1), ("20", 2)], 0);
        let mut game = rolled(&game, [3, 1]);
        go(&mut game, "5");
        assert_eq!(on(&game, "5", W), 1);
        assert_eq!(on(&game, "5", B), 0);
        assert_eq!(bar(&game, "BLACK", B), 1, "black's checker is on its bar");
    }

    #[test]
    fn a_checker_on_the_bar_enters_before_anything_else_moves() {
        // Black has a checker on the bar and a checker to move; white's home points 1-3 are open.
        let mut game = position(&[("6", 2)], &[("12", 2)], 1);
        put_on_bar(&mut game, "BLACK", B, 1);
        let game = rolled(&game, [4, 2]);
        // Black enters on 4 or 2 (its die as the point numbered from white's side); the 12 stays.
        assert_eq!(offered(&game), vec!["2".to_string(), "4".to_string()]);
        let mut game = game;
        go(&mut game, "4");
        assert_eq!(on(&game, "4", B), 1);
        assert_eq!(bar(&game, "BLACK", B), 0);
        assert_eq!(dice(&game, "BLACK"), vec![2]);
        // Now the other die may move any checker, the entered one included: 12→14, 4→6 (white).
        assert!(offered(&game).contains(&"14".to_string()));
    }

    #[test]
    fn a_blocked_entry_loses_the_turn() {
        // White has a checker on the bar; black holds white's entry points 24-19 (die 1-6) in twos
        // except none: block all six.
        let black: Vec<(&str, u32)> = ["19", "20", "21", "22", "23", "24"].iter().map(|p| (*p, 2)).collect();
        let mut game = position(&[("8", 3)], &black, 0);
        put_on_bar(&mut game, "WHITE", W, 1);
        let game = rolled(&game, [3, 1]);
        assert_eq!(game.current_player(), "BLACK", "no entry, no move: the turn passes");
        assert_eq!(bar(&game, "WHITE", W), 1);
    }

    #[test]
    fn bearing_off_needs_every_checker_home() {
        let game = position(&[("3", 2), ("7", 1)], &[("20", 2)], 0);
        let game = rolled(&game, [3, 4]);
        assert!(!offered(&game).contains(&"OFFW".to_string()), "a checker is still on 7");
    }

    #[test]
    fn a_checker_bears_off_with_its_number_and_the_tray_counts_it() {
        let game = position(&[("3", 2), ("5", 1)], &[("20", 2)], 0);
        let mut game = rolled(&game, [3, 5]);
        assert!(offered(&game).contains(&"OFFW".to_string()));
        go(&mut game, "OFFW");
        // Either die bears a checker off, so the tray alone does not say which: the die is asked.
        assert_eq!(on(&game, "OFFW", W), 0);
        let names: Vec<String> = game.choice().actions.iter().map(|a| a.name.clone()).collect();
        assert_eq!(names, vec!["OFF_3", "OFF_5"]);
        act(&mut game, "OFF_5");
        assert_eq!(on(&game, "OFFW", W), 1);
        assert_eq!(on(&game, "5", W), 0, "the 5 took the checker from the five point");
        assert_eq!(on(&game, "3", W), 2);
        assert_eq!(dice(&game, "WHITE"), vec![3]);
    }

    #[test]
    fn a_larger_die_bears_off_only_from_the_farthest_point() {
        // Checkers on 2 and 4: a 6 may only take the one from 4; a 3 may take neither (4 is not 3,
        // and 2 has the 4 behind it).
        let game = position(&[("2", 1), ("4", 1)], &[("20", 2)], 0);
        let mut game = rolled(&game, [6, 3]);
        assert!(offered(&game).contains(&"OFFW".to_string()));
        // Played with the 6 (the 3 cannot bear off): the 4 goes, the 2 stays.
        go(&mut game, "OFFW");
        assert_eq!(on(&game, "4", W), 0);
        assert_eq!(on(&game, "2", W), 1);
        assert_eq!(on(&game, "OFFW", W), 1);
        // The 3 is left: now the 2 is the farthest, so 3 > 2 bears it off.
        assert_eq!(dice(&game, "WHITE"), vec![3]);
        go(&mut game, "OFFW");
        assert_eq!(on(&game, "OFFW", W), 2);
    }

    #[test]
    fn a_larger_die_may_not_bear_off_while_a_checker_is_farther_back() {
        let game = position(&[("2", 1), ("5", 1)], &[("20", 2)], 0);
        let mut game = rolled(&game, [4, 3]);
        // 4 or 3 may not take the checker on 2 (the 5 is farther back); neither is the 5's number.
        assert!(!offered(&game).contains(&"OFFW".to_string()));
        // They can move inside the home board.
        assert!(offered(&game).contains(&"1".to_string()));
        go(&mut game, "1"); // 5→1 with the 4
        assert_eq!(on(&game, "1", W), 1);
    }

    #[test]
    fn the_last_checker_off_wins() {
        let mut game = position(&[("1", 1), ("2", 1)], &[("20", 2)], 0);
        add(&mut game, place("OFFW"), "", W, 13);
        game.work_out_ways();
        let mut game = rolled(&game, [2, 1]);
        go(&mut game, "OFFW");
        act(&mut game, "OFF_1");
        assert_eq!(on(&game, "OFFW", W), 14);
        assert!(!over(&game), "one checker is still on the board");
        assert_eq!(dice(&game, "WHITE"), vec![2]);
        go(&mut game, "OFFW");
        assert_eq!(
            game.state.phase,
            GamePhase::GameOver {
                winner: Some("WHITE".to_string())
            }
        );
        assert_eq!(on(&game, "OFFW", W), 15);
    }

    #[test]
    fn black_moves_the_other_way_and_bears_off_from_the_far_end() {
        let game = position(&[("1", 2)], &[("19", 2), ("22", 2)], 1);
        let mut game = rolled(&game, [3, 2]);
        // 19→22 with 3 and 19→21 with 2, 22→24 with 2, 22→25 off is not allowed (19 is not home...
        // it is: 19-24 is black's home, so everything is home).
        assert!(offered(&game).contains(&"21".to_string()));
        assert!(offered(&game).contains(&"OFFB".to_string()), "all black checkers are home");
        go(&mut game, "OFFB");
        assert_eq!(on(&game, "OFFB", B), 1);
    }

    #[test]
    fn a_die_that_cannot_be_played_is_lost_and_the_turn_passes() {
        // White's only checker is on 2 and black holds 1: a 1 is blocked and a 5 bears nothing off
        // (white is not all home? it is) ... use a checker that can play the 4 but not the 5.
        let game = position(&[("10", 1)], &[("5", 2), ("20", 2)], 0);
        let game = rolled(&game, [5, 4]);
        // 10→5 is blocked; 10→6 is open. After the 4, the checker on 6 meets 5→1: 6-5=1 is open.
        let mut game = game;
        assert_eq!(offered(&game), vec!["6".to_string()]);
        go(&mut game, "6");
        assert_eq!(game.current_player(), "WHITE", "the 5 can still be played, 6→1");
        go(&mut game, "1");
        assert_eq!(game.current_player(), "BLACK");
    }

    #[test]
    fn games_played_by_the_first_way_always_end_with_a_winner() {
        for seed in 0..6 {
            let mut game = load();
            game.set_seed(seed);
            let mut turns = 0;
            while !over(&game) {
                turns += 1;
                assert!(turns < 4000, "seed {seed} does not end");
                let choice = game.choice();
                if let Some(place) = choice.places.first() {
                    go(&mut game, place.name.as_str());
                } else {
                    let action = choice.actions.first().expect("something to answer");
                    act(&mut game, &action.name.clone());
                }
            }
            let white = on(&game, "OFFW", W);
            let black = on(&game, "OFFB", B);
            assert!(
                (white == 15) != (black == 15),
                "one side has all fifteen off at seed {seed}: {white} {black}"
            );
            let winner = if white == 15 { "WHITE" } else { "BLACK" };
            assert_eq!(
                game.state.phase,
                GamePhase::GameOver {
                    winner: Some(winner.to_string())
                },
                "the side with fifteen off wins, at seed {seed}"
            );
        }
    }
}
