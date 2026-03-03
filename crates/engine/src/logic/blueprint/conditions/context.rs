use std::collections::HashSet;

use crate::logic::blueprint::move_blueprint::MoveBlueprint;
use crate::logic::{Game, Piece};
use crate::shared::Position;

pub struct ConditionEvalContext<'a> {
    pub blueprint: &'a MoveBlueprint,
    pub piece: &'a Piece,
    pub original_source: &'a Position,
    pub source_position: &'a Position,
    pub valid_move_ids: &'a HashSet<u8>,
    pub game: &'a Game,
    current_player: String,
}

impl<'a> ConditionEvalContext<'a> {
    pub fn new(
        blueprint: &'a MoveBlueprint,
        piece: &'a Piece,
        original_source: &'a Position,
        source_position: &'a Position,
        valid_move_ids: &'a HashSet<u8>,
        game: &'a Game,
    ) -> Self {
        Self {
            blueprint,
            piece,
            original_source,
            source_position,
            valid_move_ids,
            game,
            current_player: game.current_player(),
        }
    }

    pub fn current_player(&self) -> &str {
        &self.current_player
    }
}
