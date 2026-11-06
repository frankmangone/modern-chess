use std::collections::{HashMap, HashSet};

use crate::logic::{Board, Game, Piece};
use crate::shared::{Effect, Position};
use crate::specs::{ConditionSpec, PieceSpec, PlayerSpec};

use super::move_blueprint::MoveBlueprint;

/// A `PieceBlueprint` is essentially a factory for piece movement calculation.
/// It stores the set of rules used to calculate the available moves, but without knowledge
/// of the actual piece position.
///
/// A piece blueprint is associated with a piece code, and contains a list of move blueprints.
/// Each move blueprint is a factory for a single move.
#[derive(Clone, Debug)]
pub struct PieceBlueprint {
    pub move_blueprints: Vec<MoveBlueprint>,
    /// Raw drop-restriction conditions (no direction transform needed).
    pub drop_restrictions: Vec<ConditionSpec>,
    /// Zone codes (names of global `POSITION` conditions) that confine this piece's legal move
    /// destinations. All zones must be satisfied; any destination outside a zone is filtered out.
    pub zone_restrictions: Vec<String>,
}

impl PieceBlueprint {
    pub fn from_spec(spec: PieceSpec, players_spec: Vec<PlayerSpec>) -> Self {
        PieceBlueprint {
            move_blueprints: spec
                .moves
                .into_iter()
                .map(|x| MoveBlueprint::from_spec(x, players_spec.clone()))
                .collect(),
            drop_restrictions: spec.drop_restrictions,
            zone_restrictions: spec.restrictions.into_iter().map(|r| r.zone).collect(),
        }
    }

    /// Returns all squares threatened by this piece from `position`.
    pub fn calculate_threats(
        &self,
        player: &str,
        position: &Position,
        game: &Game,
    ) -> HashSet<Position> {
        self.move_blueprints
            .iter()
            .flat_map(|mb| mb.calculate_threats(player, position, game))
            .collect()
    }

    /// Like `calculate_threats`, but uses an explicit `pieces` map and `board` for simulation.
    pub fn calculate_threats_with(
        &self,
        player: &str,
        position: &Position,
        pieces: &HashMap<Position, Piece>,
        board: &Board,
    ) -> HashSet<Position> {
        self.move_blueprints
            .iter()
            .flat_map(|mb| mb.calculate_threats_with(player, position, pieces, board))
            .collect()
    }

    /// Calculates the moves associated with each move blueprint.
    pub fn calculate_moves(
        &self,
        piece: &Piece,
        position: &Position,
        game: &Game,
    ) -> Option<HashMap<Position, Effect>> {
        let mut moves: HashMap<Position, Effect> = HashMap::new();

        // To evaluate move dependencies, we need to keep track of which move blueprints have valid moves.
        let mut valid_move_ids: HashSet<u8> = HashSet::new();

        for move_blueprint in &self.move_blueprints {
            // Evaluate move dependencies.
            match move_blueprint.calculate_moves(piece, position, &valid_move_ids, game) {
                Some(value) => {
                    // `value` is a vector of (Position, Vec<Effect>), where the position is the "target" position
                    // and the vector is the list of effects to be executed.
                    value.iter().for_each(|(pos, effects)| {
                        moves.insert(pos.clone(), effects.clone());
                    });

                    valid_move_ids.insert(move_blueprint.id);
                }
                None => (),
            };
        }

        // Zone-restriction post-filter: discard any move whose landing square is outside
        // one of the piece's declared zones.
        if !self.zone_restrictions.is_empty() {
            moves.retain(|target, _| {
                self.zone_restrictions.iter().all(|zone| {
                    game.check_position_condition_for_player_str(target, zone, &piece.player)
                })
            });
        }

        (!moves.is_empty()).then_some(moves)
    }
}
