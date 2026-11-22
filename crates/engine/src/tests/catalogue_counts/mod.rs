//! Pins every catalogue game's behaviour: the number of legal moves found at every ply from
//! the start, summed, walked as `examples/speed.rs` walks them. The table was recorded from
//! the specs before their rules were rewritten in test blocks; a rewrite that changes a game
//! changes its count.
#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::logic::{Game, GamePhase, GameTransition};
    use crate::shared::Position;
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
        glinski: 2 => 2637,
        go: 2 => 6561,
        go_13: 2 => 28561,
        go_19: 2 => 130321,
        gothic: 2 => 812,
        grand_chess: 2 => 4290,
        hex: 2 => 14641,
        // Recorded from the rewritten spec; at 3 plies it gave 176716 before and after the
        // rewrite, but takes most of this suite's time.
        janggi_cannon: 2 => 3350,
        king_of_the_hill: 2 => 420,
        mini_xiangqi: 2 => 49,
        pong_hau_ki: 2 => 4,
        racing_kings: 2 => 442,
        shatranj: 2 => 272,
        shogi: 3 => 26370,
        solidarity: 3 => 9156,
        three_check: 2 => 420,
        xiangqi: 3 => 82228,
    }

    /// Plays `step` on a clone of `game`, taking the first option of a promotion.
    fn play(game: &Game, step: GameTransition) -> Game {
        let mut next = game.clone();
        next.transition(step.clone())
            .unwrap_or_else(|error| panic!("{step:?} was listed but refused: {error:?}"));
        if let GamePhase::Transforming { options, .. } = &next.state.phase {
            let target = options.first().expect("a promotion offers a piece").clone();
            next.transition(GameTransition::Transform { target })
                .expect("the first promotion option is accepted");
        }
        next
    }

    /// The positions after every legal move of the player to move, counted into `moves`.
    fn successors(game: &Game, moves: &mut u64) -> Vec<Game> {
        let mut next_games = Vec::new();
        let player = game.current_player();

        let origins: Vec<Position> = game
            .state
            .pieces
            .iter()
            .filter(|(_, piece)| piece.player == player)
            .map(|(position, _)| position.clone())
            .collect();
        for origin in origins {
            let mut picked = game.clone();
            picked
                .transition(GameTransition::CalculateMoves { position: origin })
                .expect("a piece of the player to move can be picked");
            let Some(found) = picked.state.available_moves.as_ref() else {
                continue;
            };
            for destination in found.keys() {
                *moves += 1;
                let execute = GameTransition::ExecuteMove {
                    position: destination.clone(),
                };
                next_games.push(play(&picked, execute));
            }
        }

        let mut codes: Vec<String> = game
            .hand()
            .get(&player)
            .map(|held| {
                held.iter()
                    .filter(|(_, count)| **count > 0)
                    .map(|(code, _)| code.clone())
                    .collect()
            })
            .unwrap_or_default();
        codes.extend(game.rules.unlimited_supply.iter().cloned());
        codes.sort();
        codes.dedup();
        for piece_code in codes {
            let mut picked = game.clone();
            picked
                .transition(GameTransition::CalculateDrops { piece_code })
                .expect("a piece in hand can be picked");
            let Some(squares) = picked.state.available_moves.as_ref() else {
                continue;
            };
            for square in squares.keys() {
                *moves += 1;
                let execute = GameTransition::ExecuteDrop {
                    position: square.clone(),
                };
                next_games.push(play(&picked, execute));
            }
        }
        next_games
    }

    fn walk(game: &Game, plies: usize, moves: &mut u64) {
        if plies == 0 || matches!(game.state.phase, GamePhase::GameOver { .. }) {
            return;
        }
        for next in successors(game, moves) {
            walk(&next, plies - 1, moves);
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
        let game = parse_game_spec(path.to_str().expect("a UTF-8 path"))
            .map(Game::from_spec)
            .unwrap_or_else(|error| panic!("{name} loads: {error:?}"));
        let mut moves = 0;
        walk(&game, plies, &mut moves);
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
}
