//! Times legal move generation on grid games.
//!
//! For each game it walks every legal move to a fixed depth, using only the public
//! transitions, and prints one line: `<game> plies=<n> moves=<count> median_ms=<ms>`.
//! `moves` is the number of legal moves found at every ply, summed (so for `plies=2` it is
//! the moves of the first ply plus the moves of all the second-ply positions). It does not
//! depend on the machine and must not change; `median_ms` is the median of five runs.
//!
//! Run with: `cargo run --release -p modern-chess-engine --example speed`

use std::path::Path;
use std::time::Instant;

use modern_chess::logic::{Game, GamePhase, GameTransition};
use modern_chess::shared::Position;
use modern_chess::specs::parse_game_spec_str;

const RUNS: usize = 5;

/// Games to measure: spec file name and number of plies.
const GAMES: [(&str, usize); 3] = [("chess", 3), ("shogi", 3), ("go", 2)];

/// What a walk found: legal moves counted, and transitions the engine refused although it
/// had listed them (a bug signal, never skipped silently).
#[derive(Default)]
struct Tally {
    moves: u64,
    errors: u64,
}

/// Plays one move on a clone of `game`, taking the first option of a promotion.
/// Returns `None` after reporting an error the engine should not give.
fn play(game: &Game, label: &str, tally: &mut Tally, steps: &[GameTransition]) -> Option<Game> {
    let mut next = game.clone();
    for step in steps {
        if let Err(error) = next.transition(step.clone()) {
            tally.errors += 1;
            eprintln!("error: {label}: {step:?} failed: {error:?}");
            return None;
        }
    }
    if let GamePhase::Transforming { options, .. } = &next.state.phase {
        let target = options.first()?.clone();
        if let Err(error) = next.transition(GameTransition::Transform { target }) {
            tally.errors += 1;
            eprintln!("error: {label}: promotion failed: {error:?}");
            return None;
        }
    }
    Some(next)
}

/// The positions after every legal move of the player to move in `game`.
fn successors(game: &Game, label: &str, tally: &mut Tally) -> Vec<Game> {
    let mut next_games = Vec::new();
    let player = game.current_player();

    // Moves of pieces on the board.
    let origins: Vec<Position> = game
        .state
        .pieces
        .iter()
        .filter(|(_, piece)| piece.player == player)
        .map(|(position, _)| position.clone())
        .collect();
    for origin in origins {
        let mut picked = game.clone();
        let calculate = GameTransition::CalculateMoves { position: origin };
        if let Err(error) = picked.transition(calculate.clone()) {
            tally.errors += 1;
            eprintln!("error: {label}: {calculate:?} failed: {error:?}");
            continue;
        }
        let Some(moves) = picked.state.available_moves.as_ref() else {
            continue;
        };
        for destination in moves.keys() {
            let execute = GameTransition::ExecuteMove {
                position: destination.clone(),
            };
            tally.moves += 1;
            if let Some(next) = play(&picked, label, tally, &[execute]) {
                next_games.push(next);
            }
        }
    }

    // Drops, from the hand and from an unlimited supply (as the session lists them).
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
    for code in codes {
        let mut picked = game.clone();
        let calculate = GameTransition::CalculateDrops { piece_code: code };
        if let Err(error) = picked.transition(calculate.clone()) {
            tally.errors += 1;
            eprintln!("error: {label}: {calculate:?} failed: {error:?}");
            continue;
        }
        let Some(squares) = picked.state.available_moves.as_ref() else {
            continue;
        };
        for square in squares.keys() {
            let execute = GameTransition::ExecuteDrop {
                position: square.clone(),
            };
            tally.moves += 1;
            if let Some(next) = play(&picked, label, tally, &[execute]) {
                next_games.push(next);
            }
        }
    }
    next_games
}

/// Counts the legal moves at every ply down to `plies`.
fn walk(game: &Game, plies: usize, label: &str, tally: &mut Tally) {
    if plies == 0 || matches!(game.state.phase, GamePhase::GameOver { .. }) {
        return;
    }
    for next in successors(game, label, tally) {
        walk(&next, plies - 1, label, tally);
    }
}

fn load(name: &str) -> Result<Game, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("specs")
        .join(format!("{name}.json"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let spec = parse_game_spec_str(&text).map_err(|e| format!("{name}: {e:?}"))?;
    Ok(Game::from_spec(spec))
}

fn main() {
    let mut failed = false;
    for (name, plies) in GAMES {
        let game = match load(name) {
            Ok(game) => game,
            Err(error) => {
                eprintln!("error: {error}");
                failed = true;
                continue;
            }
        };
        let mut times = Vec::with_capacity(RUNS);
        let mut counts = Vec::with_capacity(RUNS);
        for _ in 0..RUNS {
            let mut tally = Tally::default();
            let started = Instant::now();
            walk(&game, plies, name, &mut tally);
            times.push(started.elapsed().as_secs_f64() * 1000.0);
            counts.push((tally.moves, tally.errors));
        }
        times.sort_by(f64::total_cmp);
        let median = times[RUNS / 2];
        let (moves, errors) = counts[0];
        if counts.iter().any(|count| *count != (moves, errors)) {
            eprintln!("error: {name}: the counts changed between runs: {counts:?}");
            failed = true;
        }
        if errors > 0 {
            eprintln!("error: {name}: {errors} listed moves were refused");
            failed = true;
        }
        println!("{name} plies={plies} moves={moves} median_ms={median:.1}");
    }
    if failed {
        std::process::exit(1);
    }
}
