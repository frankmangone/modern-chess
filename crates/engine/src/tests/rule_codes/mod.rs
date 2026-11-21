#[cfg(test)]
mod tests {
    use crate::logic::blueprint::move_blueprint::{ConditionKind, SideEffectKind};
    use crate::logic::{CellConditionKind, DropRestrictionKind, Game, WinConditionKind};
    use crate::specs::parse_game_spec;

    fn load(name: &str) -> Game {
        parse_game_spec(&format!("./specs/{name}.json"))
            .map(Game::from_spec)
            .expect("Failed to load spec")
    }

    fn conditions_of<'a>(game: &'a Game, code: &str) -> Vec<&'a ConditionKind> {
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
            .map(|condition| &condition.kind)
            .collect()
    }

    #[test]
    fn pawn_promotion_condition_is_named() {
        let game = load("chess");
        assert!(
            conditions_of(&game, "PAWN").contains(&&ConditionKind::Named("REACH_END".to_string()))
        );
    }

    #[test]
    fn castling_path_condition_is_built_in() {
        let game = load("chess");
        assert!(conditions_of(&game, "KING").contains(&&ConditionKind::PathPieceCount));
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
    fn declared_condition_type_is_parsed() {
        let game = load("chess");
        assert_eq!(
            game.rules.conditions["REACH_END"].kind,
            CellConditionKind::Position
        );
    }

    #[test]
    fn drop_restriction_is_parsed() {
        let game = load("shogi");
        let kinds: Vec<&DropRestrictionKind> = game
            .rules
            .blueprints
            .values()
            .flat_map(|bp| bp.drop_restrictions.iter().map(|r| &r.kind))
            .collect();
        assert!(kinds.contains(&&DropRestrictionKind::AllyOnFile));
    }
}
