//! How long move generation takes, to compare the engine before and after a change. Run with
//! `cargo test --release -p modern-chess-engine timing -- --ignored --nocapture`.
#[cfg(test)]
mod tests {
    use std::time::Instant;

    use crate::logic::{Game, GameTransition};
    use crate::specs::parse_game_spec;

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
        let chess = load("./specs/chess.json");
        let white: Vec<_> = chess
            .state
            .pieces
            .iter()
            .filter(|(_, piece)| piece.player == "WHITE")
            .map(|(position, _)| position.clone())
            .collect();
        let chess_moves = mean_micros(|| {
            let mut game = chess.clone();
            for position in &white {
                game.transition(GameTransition::CalculateMoves {
                    position: position.clone(),
                })
                .unwrap();
            }
            assert!(game.any_legal_moves());
        });

        let mut go = load("./specs/go_19.json");
        let mut placed = 0;
        for i in 0u32..400 {
            if placed == 60 {
                break;
            }
            let point = i * 7 % 361;
            let position = vec![(point % 19) as u8, (point / 19) as u8];
            let player = go.current_player();
            if !go
                .compute_drop_squares("STONE", &player)
                .contains_key(&position)
            {
                continue;
            }
            go.transition(GameTransition::CalculateDrops {
                piece_code: "STONE".to_string(),
            })
            .unwrap();
            go.transition(GameTransition::ExecuteDrop { position })
                .unwrap();
            placed += 1;
        }
        assert!(go.state.pieces.len() > 40);
        let go_drops = mean_micros(|| {
            let player = go.current_player();
            assert!(!go.compute_drop_squares("STONE", &player).is_empty());
        });

        println!("chess: every White piece's moves + any_legal_moves: {chess_moves:.1} µs");
        println!("go 19×19, 60 stones: every legal stone placement: {go_drops:.1} µs");
    }
}
