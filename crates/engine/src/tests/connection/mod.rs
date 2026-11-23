//! The CONNECTION win condition: one chain of a player's pieces touching enough zones. Played
//! on a square board, to show the rule is not tied to hex boards.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Game, GamePhase, Piece};
    use crate::specs::{parse_game_spec_str, GameSpecError};
    use crate::tests::support::{drop, place_action};

    fn column(x: u8) -> Vec<[u8; 2]> {
        (0..5).map(|y| [x, y]).collect()
    }

    fn row(y: u8) -> Vec<[u8; 2]> {
        (0..5).map(|x| [x, y]).collect()
    }

    /// A 5×5 board where A joins the left and right columns and B the bottom and top rows.
    fn spec(win: Value) -> String {
        json!({
            "name": "CONNECT TEST",
            "board": { "dimensions": [5, 5] },
            "leader": [],
            "turns": { "order": ["A", "B"] },
            "players": [
                { "name": "A", "direction": [[1, 0], [0, 1]], "starting_positions": [] },
                { "name": "B", "direction": [[1, 0], [0, 1]], "starting_positions": [] }
            ],
            "pieces": [{ "code": "STONE", "name": "stone", "moves": [] }],
            "actions": [place_action("STONE")],
            "zones": [
                { "name": "LEFT", "places": { "A": column(0), "B": row(0) } },
                { "name": "RIGHT", "places": { "A": column(4), "B": row(4) } }
            ],
            "win_conditions": [win]
        })
        .to_string()
    }

    fn connection(through: &str) -> Value {
        json!({ "type": "CONNECTION", "through": through, "zones": ["LEFT", "RIGHT"], "pieces": ["STONE"] })
    }

    fn game(win: Value) -> Game {
        parse_game_spec_str(&spec(win))
            .map(Game::from_spec)
            .expect("a valid spec")
    }

    /// Puts A's stones on `cells`, then has A drop one more on `last`.
    fn a_plays(game: &mut Game, cells: &[(u8, u8)], last: (u8, u8)) {
        for &(x, y) in cells {
            game.state
                .pieces
                .insert(pos![x, y], Piece::new("STONE".to_string(), "A".to_string()));
        }
        drop(game, "STONE", pos![last.0, last.1]).unwrap();
    }

    fn a_won(game: &Game) -> bool {
        game.state.phase
            == GamePhase::GameOver {
                winner: Some("A".to_string()),
            }
    }

    #[test]
    fn a_chain_across_wins() {
        let mut game = game(connection("orthogonal"));
        a_plays(&mut game, &[(0, 2), (1, 2), (2, 2), (3, 2)], (4, 2));
        assert!(a_won(&game));
    }

    #[test]
    fn a_chain_one_short_does_not_win() {
        let mut game = game(connection("orthogonal"));
        a_plays(&mut game, &[(0, 2), (1, 2)], (2, 2));
        assert!(!matches!(game.state.phase, GamePhase::GameOver { .. }));
    }

    #[test]
    fn a_diagonal_chain_does_not_connect_orthogonally() {
        let diagonal = [(0, 0), (1, 1), (2, 2), (3, 3)];
        let mut orthogonal = game(connection("orthogonal"));
        a_plays(&mut orthogonal, &diagonal, (4, 4));
        assert!(!a_won(&orthogonal));

        let mut adjacent = game(connection("adjacent"));
        a_plays(&mut adjacent, &diagonal, (4, 4));
        assert!(a_won(&adjacent));
    }

    #[test]
    fn the_opponents_sides_do_not_count() {
        let mut game = game(connection("orthogonal"));
        a_plays(&mut game, &[(2, 0), (2, 1), (2, 2), (2, 3)], (2, 4));
        assert!(!matches!(game.state.phase, GamePhase::GameOver { .. }));
    }

    #[test]
    fn at_least_one_of_two_zones() {
        let mut win = connection("orthogonal");
        win["at_least"] = json!(1);
        let mut game = game(win);
        a_plays(&mut game, &[], (0, 2));
        assert!(a_won(&game));
    }

    fn refused(win: Value) -> bool {
        matches!(
            parse_game_spec_str(&spec(win)),
            Err(GameSpecError::InvalidWinCondition(_))
        )
    }

    #[test]
    fn unknown_zone_is_refused() {
        let mut win = connection("orthogonal");
        win["zones"] = json!(["LEFT", "NOWHERE"]);
        assert!(refused(win));
    }

    #[test]
    fn unknown_neighbourhood_is_refused() {
        assert!(refused(connection("sideways")));
    }

    #[test]
    fn at_least_zero_is_refused() {
        let mut win = connection("orthogonal");
        win["at_least"] = json!(0);
        assert!(refused(win));
    }

    #[test]
    fn at_least_above_the_zones_is_refused() {
        let mut win = connection("orthogonal");
        win["at_least"] = json!(3);
        assert!(refused(win));
    }

    #[test]
    fn a_zone_named_twice_is_refused() {
        let mut win = connection("orthogonal");
        win["zones"] = json!(["LEFT", "LEFT"]);
        assert!(refused(win));
    }

    #[test]
    fn a_missing_neighbourhood_says_through_is_required() {
        let mut win = connection("orthogonal");
        win.as_object_mut().unwrap().remove("through");
        let error = parse_game_spec_str(&spec(win))
            .err()
            .expect("refused")
            .to_string();
        assert!(error.contains("`through` is required"), "{error}");
    }
}
