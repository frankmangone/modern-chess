//! Hex: RED joins its two sides (x = 0 and x = 10), BLUE the other two (y = 0 and y = 10).
#[cfg(test)]
mod tests {
    use crate::logic::{Game, GamePhase, GameTransition, Piece};
    use crate::specs::parse_game_spec;

    fn load() -> Game {
        parse_game_spec("./specs/hex.json")
            .map(Game::from_spec)
            .expect("Failed to load hex spec")
    }

    fn stones(game: &mut Game, player: &str, cells: impl IntoIterator<Item = (u8, u8)>) {
        for (x, y) in cells {
            game.state.pieces.insert(
                pos![x, y],
                Piece::new("STONE".to_string(), player.to_string()),
            );
        }
    }

    fn drop_stone(game: &mut Game, x: u8, y: u8) {
        game.transition(GameTransition::CalculateDrops {
            piece_code: "STONE".to_string(),
        })
        .unwrap();
        game.transition(GameTransition::ExecuteDrop {
            position: pos![x, y],
        })
        .unwrap();
    }

    fn winner(game: &Game) -> Option<String> {
        match &game.state.phase {
            GamePhase::GameOver { winner } => winner.clone(),
            _ => None,
        }
    }

    #[test]
    fn hex_has_121_empty_cells_and_red_starts() {
        let game = load();
        assert_eq!(game.board.all_positions().len(), 121);
        assert!(game.state.pieces.is_empty());
        assert_eq!(game.current_player(), "RED");
    }

    #[test]
    fn red_wins_by_joining_its_sides() {
        let mut game = load();
        stones(&mut game, "RED", (0..=9).map(|x| (x, 5)));
        drop_stone(&mut game, 10, 5);
        assert_eq!(winner(&game).as_deref(), Some("RED"));
    }

    #[test]
    fn blue_wins_by_joining_its_sides() {
        let mut game = load();
        game.state.current_turn = 1;
        stones(&mut game, "BLUE", (0..=9).map(|y| (5, y)));
        drop_stone(&mut game, 5, 10);
        assert_eq!(winner(&game).as_deref(), Some("BLUE"));
    }

    #[test]
    fn red_on_blues_sides_does_not_win() {
        let mut game = load();
        stones(&mut game, "RED", (0..=9).map(|y| (5, y)));
        drop_stone(&mut game, 5, 10);
        assert!(!matches!(game.state.phase, GamePhase::GameOver { .. }));
    }

    #[test]
    fn every_empty_cell_is_a_legal_drop() {
        let mut game = load();
        game.transition(GameTransition::CalculateDrops {
            piece_code: "STONE".to_string(),
        })
        .unwrap();
        let drops = game
            .state
            .available_moves
            .as_ref()
            .map_or(0, |moves| moves.len());
        assert_eq!(drops, 121);
    }
}
