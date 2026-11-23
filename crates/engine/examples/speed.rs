//! Times legal move generation on grid games.
//!
//! For each game it walks every legal move to a fixed depth, using only the public protocol,
//! and prints one line: `<game> plies=<n> moves=<count> median_ms=<ms>`. `moves` is the number
//! of legal moves found at every ply, summed (so for `plies=2` it is the moves of the first ply
//! plus the moves of all the second-ply positions). A move is a distinct (action, piece place
//! or hand code, target) pair of the turn's ways, so a promotion's options count once, as
//! `catalogue_counts` counts them. It does not depend on the machine and must not change;
//! `median_ms` is the median of five runs.
//!
//! The turn's ways are worked out when the turn starts, so reaching a position works out its
//! moves: the walk plays the first way of each pair to go a ply deeper, and counts the last
//! ply's pairs without playing them.
//!
//! Run with: `cargo run --release -p modern-chess-engine --example speed`

use std::collections::HashSet;
use std::path::Path;
use std::time::Instant;

use modern_chess::logic::{Answer, Game, GamePhase, GameTransition, Way};
use modern_chess::specs::parse_game_spec_str;

const RUNS: usize = 5;

/// Games to measure: spec file name and number of plies.
const GAMES: [(&str, usize); 3] = [("chess", 3), ("shogi", 3), ("go", 2)];

/// What a walk found: legal moves counted, and ways the engine refused although it had listed
/// them (a bug signal, never skipped silently).
#[derive(Default)]
struct Tally {
    moves: u64,
    errors: u64,
}

/// The first way of each distinct (action, first answer, target place) pair, in the order of
/// the ways. A way with no answers (a pass) is no move.
fn first_of_each_pair(ways: &[Way]) -> Vec<&Way> {
    let mut seen: HashSet<(&str, &Answer, Option<&Answer>)> = HashSet::new();
    ways.iter()
        .filter(|way| {
            let Some(first) = way.answers.first() else {
                return false;
            };
            let target = way
                .answers
                .get(1)
                .filter(|answer| matches!(answer, Answer::Place(_)));
            seen.insert((way.action.as_str(), first, target))
        })
        .collect()
}

/// Plays `way` on a clone of `game`, answering its choices one by one. The game's ways were
/// taken out, so the clone does not copy them: only the way being played is put back.
/// Returns `None` after reporting an answer the engine should not refuse.
fn play(game: &Game, way: &Way, label: &str, tally: &mut Tally) -> Option<Game> {
    let mut next = game.clone();
    next.state.ways = Some(vec![way.clone()]);
    for answer in &way.answers {
        if let Err(error) = next.transition(GameTransition::Choose(answer.clone())) {
            tally.errors += 1;
            eprintln!("error: {label}: {answer:?} of {way:?} failed: {error:?}");
            return None;
        }
    }
    if next.history().len() != game.history().len() + 1 {
        tally.errors += 1;
        eprintln!("error: {label}: {way:?} was not played");
        return None;
    }
    Some(next)
}

/// Counts the legal moves at every ply down to `plies`.
fn walk(game: &mut Game, plies: usize, label: &str, tally: &mut Tally) {
    if plies == 0 || matches!(game.state.phase, GamePhase::GameOver { .. }) {
        return;
    }
    let ways = game.state.ways.take().unwrap_or_default();
    let firsts = first_of_each_pair(&ways);
    tally.moves += firsts.len() as u64;
    if plies > 1 {
        for way in firsts {
            if let Some(mut next) = play(game, way, label, tally) {
                walk(&mut next, plies - 1, label, tally);
            }
        }
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
            // The first position's moves are worked out inside the timing, as every other's.
            let mut start = game.clone();
            start.work_out_ways();
            walk(&mut start, plies, name, &mut tally);
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
            eprintln!("error: {name}: {errors} listed ways were refused");
            failed = true;
        }
        println!("{name} plies={plies} moves={moves} median_ms={median:.1}");
    }
    if failed {
        std::process::exit(1);
    }
}
