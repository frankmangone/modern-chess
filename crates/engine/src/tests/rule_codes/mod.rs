#[cfg(test)]
mod tests {
    use crate::logic::blueprint::move_blueprint::SideEffectKind;
    use crate::logic::blueprint::test_block::{Owner, Region, Test};
    use crate::logic::{Game, WinConditionKind};
    use crate::specs::parse_game_spec;

    fn load(name: &str) -> Game {
        parse_game_spec(&format!("./specs/{name}.json"))
            .map(Game::from_spec)
            .expect("Failed to load spec")
    }

    /// The tests of a piece's moves, their modifiers and their actions.
    fn tests_of<'a>(game: &'a Game, code: &str) -> Vec<&'a Test> {
        let blueprint = game.rules.blueprints.get(code).expect("piece exists");
        blueprint
            .move_blueprints
            .iter()
            .flat_map(|mv| {
                mv.conditions
                    .iter()
                    .chain(mv.modifiers.iter().flat_map(|m| m.conditions.iter()))
                    .chain(mv.actions.values().flat_map(|a| a.conditions.iter()))
            })
            .collect()
    }

    #[test]
    fn pawn_promotion_condition_is_named() {
        let game = load("chess");
        assert!(tests_of(&game, "PAWN")
            .iter()
            .any(|test| matches!(test, Test::InZone { zone, .. } if zone == "REACH_END")));
    }

    #[test]
    fn castling_path_condition_is_built_in() {
        let game = load("chess");
        assert!(tests_of(&game, "KING").iter().any(|test| matches!(
            test,
            Test::Count {
                region: Region::Path { .. },
                ..
            }
        )));
    }

    #[test]
    fn capture_side_effect_is_parsed() {
        let game = load("atomic");
        let explodes = game.rules.blueprints.values().any(|bp| {
            bp.move_blueprints.iter().any(|mv| {
                mv.actions
                    .values()
                    .flat_map(|a| a.side_effects.iter())
                    .any(|se| se.kind == SideEffectKind::Explode)
            })
        });
        assert!(explodes);
    }

    #[test]
    fn win_condition_is_parsed() {
        let game = load("king_of_the_hill");
        assert_eq!(
            game.rules.win_conditions[0].kind,
            WinConditionKind::PieceInZone
        );
    }

    #[test]
    fn declared_zone_is_parsed() {
        let game = load("chess");
        let reach_end = &game.rules.zones["REACH_END"];
        assert!(reach_end.places.contains_key("WHITE"));
        assert!(reach_end.places.contains_key("BLACK"));
    }

    #[test]
    fn drop_condition_is_parsed() {
        let game = load("shogi");
        let pawn = game.rules.blueprints.get("PAWN").expect("piece exists");
        let no_ally_pawn_on_the_file = pawn.drop_conditions.iter().any(|test| {
            matches!(test, Test::Not(inner) if matches!(
                inner.as_ref(),
                Test::Count { region: Region::Line { .. }, filter, .. }
                    if filter.owner == Owner::Ally && filter.codes == ["PAWN"]
            ))
        });
        assert!(no_ally_pawn_on_the_file);
    }
}
