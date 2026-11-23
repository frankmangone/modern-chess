//! Pong hau k'i: five points, seven links, two pieces each. A piece steps along a link to the
//! empty point, a player who cannot move loses, and a threefold repetition is a draw.
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crate::logic::{Game, GamePhase};
    use crate::shared::Position;
    use crate::specs::{parse_game_spec, GameSpec, MapSpec, PlaceKind};
    use crate::tests::support::{destinations, go, pick};

    const SPEC: &str = "./specs/pong_hau_ki.json";

    fn spec() -> GameSpec {
        parse_game_spec(SPEC).expect("the Pong hau k'i spec")
    }

    fn load() -> Game {
        Game::from_spec(spec())
    }

    fn at(name: &str) -> Position {
        Position::from(name)
    }

    fn play(game: &mut Game, from: &str, to: &str) {
        pick(game, at(from)).unwrap_or_else(|e| panic!("cannot select {from}: {e:?}"));
        go(game, at(to)).unwrap_or_else(|e| panic!("cannot move {from} to {to}: {e:?}"));
    }

    fn owner(game: &Game, place: &str) -> Option<String> {
        game.piece_at_position(&at(place))
            .map(|piece| piece.player.to_string())
    }

    fn linked<'a>(map: &'a MapSpec, from: &str) -> BTreeSet<&'a str> {
        let from = map.index(from).expect("a place");
        map.links["next"][from]
            .iter()
            .map(|&to| map.places[to].name.as_str())
            .collect()
    }

    #[test]
    fn five_points_and_seven_links() {
        let spec = spec();
        let map = spec.board.map.as_deref().expect("a map board");
        let points: Vec<(&str, [f64; 2])> = map
            .places
            .iter()
            .map(|place| {
                assert_eq!(place.kind, PlaceKind::Point, "{}", place.name);
                (place.name.as_str(), place.at)
            })
            .collect();
        assert_eq!(
            points,
            [
                ("top-left", [0.0, 2.0]),
                ("top-right", [2.0, 2.0]),
                ("centre", [1.0, 1.0]),
                ("bottom-left", [0.0, 0.0]),
                ("bottom-right", [2.0, 0.0]),
            ]
        );

        let names = |list: &[&'static str]| list.iter().copied().collect::<BTreeSet<_>>();
        assert_eq!(
            linked(map, "centre"),
            names(&["top-left", "top-right", "bottom-left", "bottom-right"])
        );
        assert_eq!(linked(map, "top-left"), names(&["centre", "bottom-left"]));
        assert_eq!(linked(map, "top-right"), names(&["centre", "bottom-right"]));
        assert_eq!(
            linked(map, "bottom-left"),
            names(&["centre", "top-left", "bottom-right"])
        );
        assert_eq!(
            linked(map, "bottom-right"),
            names(&["centre", "top-right", "bottom-left"])
        );
    }

    #[test]
    fn black_starts_on_top_white_on_the_bottom_and_white_moves_first() {
        let game = load();
        assert_eq!(game.current_player(), "WHITE");
        for place in ["top-left", "top-right"] {
            assert_eq!(owner(&game, place).as_deref(), Some("BLACK"), "{place}");
        }
        for place in ["bottom-left", "bottom-right"] {
            assert_eq!(owner(&game, place).as_deref(), Some("WHITE"), "{place}");
        }
        assert!(game.piece_at_position(&at("centre")).is_none());
        assert_eq!(game.state.pieces.len(), 4);
    }

    #[test]
    fn whites_first_moves_are_to_the_centre_from_either_piece() {
        for from in ["bottom-left", "bottom-right"] {
            let mut game = load();
            pick(&mut game, at(from)).expect("a white piece can be selected");
            let destinations: Vec<String> = destinations(&game);
            assert_eq!(destinations, ["centre"], "{from}");
        }
    }

    #[test]
    fn a_player_who_cannot_move_loses() {
        let mut game = load();
        for (from, to) in [
            ("bottom-left", "centre"),
            ("top-left", "bottom-left"),
            ("centre", "top-left"),
            ("bottom-left", "centre"),
            ("bottom-right", "bottom-left"),
        ] {
            play(&mut game, from, to);
            assert!(
                !matches!(game.state.phase, GamePhase::GameOver { .. }),
                "after {from} to {to}"
            );
        }

        play(&mut game, "top-right", "bottom-right");

        assert_eq!(
            game.state.phase,
            GamePhase::GameOver {
                winner: Some("BLACK".to_string())
            }
        );
    }

    #[test]
    fn the_same_position_three_times_is_a_draw() {
        let mut game = load();
        let round = [
            ("bottom-left", "centre"),
            ("top-left", "bottom-left"),
            ("centre", "top-left"),
            ("bottom-left", "centre"),
            ("top-left", "bottom-left"),
            ("centre", "top-left"),
        ];
        // No rule of this game reads whether a piece has moved, so the position record leaves
        // that out: the position after the round's first move comes again after its seventh
        // and its thirteenth. (With the has-moved flag in the record, the draw came a ply later,
        // on the 14th.)
        let mut played = 0;
        'laps: for lap in 0..3 {
            assert!(
                !matches!(game.state.phase, GamePhase::GameOver { .. }),
                "still playing at lap {lap}"
            );
            for (from, to) in round {
                play(&mut game, from, to);
                played += 1;
                if matches!(game.state.phase, GamePhase::GameOver { .. }) {
                    break 'laps;
                }
            }
        }
        assert_eq!(game.state.phase, GamePhase::GameOver { winner: None });
        assert_eq!(played, 13, "drawn on the third lap's first move");
    }
}
