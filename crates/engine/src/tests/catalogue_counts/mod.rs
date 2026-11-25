//! Pins every catalogue game's behaviour: the number of legal moves found at every ply from
//! the start, summed, walked as `examples/speed.rs` walks them. A move is a distinct (piece
//! place or hand code, target) pair of an action's ways. The table was recorded from
//! the specs before their rules were rewritten in test blocks; a rewrite that changes a game
//! changes its count.
#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::Path;

    use crate::logic::{Answer, Game, GamePhase, GameTransition, Way};
    use crate::specs::parse_game_spec;

    /// Every catalogue spec, with the plies walked and the legal moves summed over every ply.
    /// One test per game, so they run side by side.
    macro_rules! counts {
        ($($name:ident: $plies:expr => $moves:expr,)*) => {
            /// The names of the games in the table.
            const GAMES: &[&str] = &[$(stringify!($name)),*];
            $(
                #[test]
                fn $name() {
                    assert_eq!(count(stringify!($name), $plies), $moves);
                }
            )*
        };
    }

    counts! {
        _4player_chess: 2 => 195,
        ataxx: 2 => 272,
        atomic: 2 => 420,
        capablanca: 2 => 812,
        chaturanga: 2 => 272,
        checkers: 2 => 56,
        chess: 3 => 9322,
        courier: 2 => 420,
        crazyhouse: 3 => 9322,
        flying_knight: 3 => 3293,
        fox_and_geese: 4 => 770,
        glinski: 2 => 2637,
        go: 2 => 6561,
        go_13: 2 => 28561,
        go_19: 2 => 130321,
        gothic: 2 => 812,
        grand_chess: 2 => 4290,
        hex: 2 => 14641,
        // Recorded from the rewritten spec; at 3 plies it gave 176716 before and after the
        // rewrite, but takes most of this suite's time.
        janggi_cannon: 2 => 1174,
        kalah: 4 => 1168,
        king_of_the_hill: 2 => 420,
        mini_xiangqi: 2 => 49,
        pong_hau_ki: 2 => 4,
        racing_kings: 2 => 442,
        // The walk counts moves that answer something; the round's opening action answers
        // nothing and the throws wait for both seats, so it reads none (the test below checks the
        // opening instead). `rps.rs` in the session crate pins the game.
        rock_paper_scissors: 2 => 0,
        shatranj: 2 => 272,
        shogi: 3 => 26370,
        solidarity: 3 => 9156,
        three_check: 2 => 420,
        xiangqi: 3 => 82228,
    }

    /// What a counted move is: the action, the place of the piece (or the code from the hand,
    /// or a supply's square) it starts from, and the place it goes to.
    type Pair = (String, Answer, Option<Answer>);

    /// The first way of each distinct pair, in the order of the ways. A promotion's options are
    /// one pair, so its first way is its first option. A way with no answers (a pass) is no
    /// move.
    fn first_of_each_pair(ways: &[Way]) -> Vec<&Way> {
        let mut seen: HashSet<Pair> = HashSet::new();
        ways.iter()
            .filter(|way| {
                let Some(first) = way.answers.first() else {
                    return false;
                };
                let target = way
                    .answers
                    .get(1)
                    .filter(|answer| matches!(answer, Answer::Place(_)));
                seen.insert((way.action.clone(), first.clone(), target.cloned()))
            })
            .collect()
    }

    /// Plays `way` on a clone of `game` (whose ways were taken out, so the clone does not copy
    /// them), answering its choices one by one.
    fn play(game: &Game, way: &Way) -> Game {
        let mut next = game.clone();
        next.state.ways = Some(vec![way.clone()]);
        for answer in &way.answers {
            next.transition(GameTransition::Choose(answer.clone()))
                .unwrap_or_else(|error| panic!("{answer:?} of {way:?} was refused: {error:?}"));
        }
        assert_eq!(
            next.history().len(),
            game.history().len() + 1,
            "{way:?} was played"
        );
        next
    }

    /// Counts the legal moves at every ply down to `plies` into `moves`, playing the first way
    /// of each pair to go deeper.
    fn walk(game: &mut Game, plies: usize, moves: &mut u64) {
        if plies == 0 || matches!(game.state.phase, GamePhase::GameOver { .. }) {
            return;
        }
        let ways = game.state.ways.take().unwrap_or_default();
        let firsts = first_of_each_pair(&ways);
        *moves += firsts.len() as u64;
        if plies > 1 {
            for way in firsts {
                walk(&mut play(game, way), plies - 1, moves);
            }
        }
    }

    fn specs_folder() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("specs")
    }

    /// The legal moves of the game `name` (a test name: a leading `_` is dropped), summed over
    /// every ply down to `plies`.
    fn count(name: &str, plies: usize) -> u64 {
        let name = name.trim_start_matches('_');
        let path = specs_folder().join(format!("{name}.json"));
        let mut game = parse_game_spec(path.to_str().expect("a UTF-8 path"))
            .map(Game::from_spec)
            .unwrap_or_else(|error| panic!("{name} loads: {error:?}"));
        let mut moves = 0;
        walk(&mut game, plies, &mut moves);
        moves
    }

    #[test]
    fn every_game_keeps_its_move_counts() {
        let mut on_disk: Vec<String> = std::fs::read_dir(specs_folder())
            .expect("the specs folder is readable")
            .filter_map(|entry| {
                let name = entry.ok()?.file_name().into_string().ok()?;
                name.strip_suffix(".json").map(str::to_string)
            })
            .collect();
        on_disk.sort();
        let mut listed: Vec<String> = GAMES
            .iter()
            .map(|name| name.trim_start_matches('_').to_string())
            .collect();
        listed.sort();
        assert_eq!(
            on_disk, listed,
            "every catalogue spec has a row in the table"
        );
    }

    #[test]
    fn rock_paper_scissors_opens_with_its_one_action() {
        let path = specs_folder().join("rock_paper_scissors.json");
        let mut game = parse_game_spec(path.to_str().expect("a UTF-8 path"))
            .map(Game::from_spec)
            .expect("the game loads");
        let ways = game.state.ways.take().unwrap_or_default();
        let opening: Vec<_> = ways.iter().map(|way| way.action.as_str()).collect();
        assert_eq!(opening, ["PLAY"]);
    }
}
