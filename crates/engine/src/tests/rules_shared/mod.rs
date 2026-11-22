#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::logic::{Game, Piece};
    use crate::specs::parse_game_spec;
    use crate::tests::support::{go, pick};

    fn load(path: &str) -> Game {
        parse_game_spec(path)
            .map(Game::from_spec)
            .expect("Failed to load spec")
    }

    #[test]
    fn a_cloned_game_shares_its_rules() {
        let game = load("./specs/chess.json");
        assert!(Arc::ptr_eq(&game.rules, &game.clone().rules));
    }

    #[test]
    fn changing_a_clones_rules_leaves_the_original() {
        let game = load("./specs/chess.json");
        let mut copy = game.clone();
        Arc::make_mut(&mut copy.rules).no_opponent_check = !game.rules.no_opponent_check;
        assert_ne!(copy.rules.no_opponent_check, game.rules.no_opponent_check);
    }

    #[test]
    fn eliminating_a_player_changes_only_that_game() {
        // The set-up of `game_over::test_nplayer_checkmated_player_is_eliminated`: BLACK
        // checkmates WHITE in a three-player game, which removes WHITE from the turn order.
        let original = load("./src/tests/game_over/spec_3player.json");
        let mut game = original.clone();
        for (position, code, player) in [
            (pos![0, 0], "KING_PIECE", "WHITE"),
            (pos![2, 1], "SLIDER", "BLACK"),
            (pos![3, 0], "SLIDER", "BLACK"),
            (pos![7, 7], "KING_PIECE", "RED"),
        ] {
            game.state
                .pieces
                .insert(position, Piece::new(code.to_string(), player.to_string()));
        }
        pick(&mut game, pos![3, 0]).unwrap();
        go(&mut game, pos![2, 0]).unwrap();

        assert_eq!(game.state.turn_order.len(), 2);
        assert_eq!(original.state.turn_order.len(), 3);
    }
}
