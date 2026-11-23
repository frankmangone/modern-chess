//! How long move generation takes, to compare the engine before and after a change. Run with
//! `cargo test --release -p modern-chess-engine timing -- --ignored --nocapture`.
#[cfg(test)]
mod tests {
    use std::time::Instant;

    use crate::logic::Game;
    use crate::specs::parse_game_spec;
    use crate::tests::support::{drop, drop_squares};

    const RUNS: u32 = 200;

    fn load(path: &str) -> Game {
        parse_game_spec(path)
            .map(Game::from_spec)
            .expect("spec loads")
    }

    fn mean_micros(mut work: impl FnMut()) -> f64 {
        work();
        let start = Instant::now();
        for _ in 0..RUNS {
            work();
        }
        start.elapsed().as_secs_f64() * 1e6 / f64::from(RUNS)
    }

    #[test]
    #[ignore]
    fn move_generation_timing() {
        // Every White piece's moves are the turn's ways, worked out once per turn.
        let chess = load("./specs/chess.json");
        let chess_moves = mean_micros(|| {
            let mut game = chess.clone();
            game.work_out_ways();
            assert!(game
                .state
                .ways
                .as_ref()
                .is_some_and(|ways| !ways.is_empty()));
        });

        let mut go = load("./specs/go_19.json");
        let mut placed = 0;
        for i in 0u32..400 {
            if placed == 60 {
                break;
            }
            let point = i * 7 % 361;
            let position = pos![(point % 19) as u8, (point / 19) as u8];
            if !drop_squares(&go, "STONE").contains(&position) {
                continue;
            }
            drop(&mut go, "STONE", position).unwrap();
            placed += 1;
        }
        assert!(go.state.pieces.len() > 40);
        let go_drops = mean_micros(|| {
            assert!(!go.compute_ways().is_empty());
        });

        println!("chess: the turn's ways (every White piece's moves): {chess_moves:.1} µs");
        println!(
            "go 19×19, 60 stones: the turn's ways (every legal stone placement): {go_drops:.1} µs"
        );
    }
}
