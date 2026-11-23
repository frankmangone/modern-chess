//! A win by connection, written as an end check: one chain of a player's pieces touching enough
//! zones. Played on a square board, to show the rule is not tied to hex boards.
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::logic::{Game, GamePhase, Piece};
    use crate::specs::parse_game_spec_str;
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
            "format_version": 1,
            "name": "CONNECT TEST",
            "board": { "dimensions": [5, 5] },
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
            "end_checks": [win]
        })
        .to_string()
    }

    /// An end check: A wins when one of its chains (joined `through`) touches `at_least` of the
    /// zones named, all of them when it is `None`.
    fn connecting(through: &str, zones: &[&str], at_least: Option<usize>) -> Value {
        let touches = |zone: &str| {
            json!({
                "test": "SOME",
                "in": { "region": "GROUP", "of": { "place": "EACH" }, "through": through,
                        "where": { "codes": ["STONE"] } },
                "that": { "test": "IN_ZONE", "at": { "place": "EACH" }, "zone": zone }
            })
        };
        let all = at_least.is_none_or(|n| n == zones.len());
        let chain = json!({
            "test": if all { "ALL" } else { "ANY" },
            "of": zones.iter().map(|zone| touches(zone)).collect::<Vec<_>>()
        });
        json!({
            "when": {
                "test": "SOME",
                "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": ["STONE"] } },
                "that": chain
            },
            "then": { "outcome": "WIN" }
        })
    }

    fn connection(through: &str) -> Value {
        connecting(through, &["LEFT", "RIGHT"], None)
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
        let mut game = game(connecting("orthogonal", &["LEFT", "RIGHT"], Some(1)));
        a_plays(&mut game, &[], (0, 2));
        assert!(a_won(&game));
    }

    fn refused(win: Value) -> bool {
        parse_game_spec_str(&spec(win)).is_err()
    }

    #[test]
    fn unknown_zone_is_refused() {
        assert!(refused(connecting(
            "orthogonal",
            &["LEFT", "NOWHERE"],
            None
        )));
    }

    #[test]
    fn unknown_neighbourhood_is_refused() {
        assert!(refused(connection("sideways")));
    }
}
