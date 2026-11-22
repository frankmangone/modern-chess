use std::collections::{HashMap, HashSet};

use crate::logic::{Board, Game, Labels, Piece};
use crate::shared::{Effect, Position};
use crate::specs::{PieceSpec, PlayerSpec};

use super::move_blueprint::MoveBlueprint;
use super::test_block::Test;

/// A `PieceBlueprint` is essentially a factory for piece movement calculation.
/// It stores the set of rules used to calculate the available moves, but without knowledge
/// of the actual piece position.
///
/// A piece blueprint is associated with a piece code, and contains a list of move blueprints.
/// Each move blueprint is a factory for a single move.
#[derive(Clone, Debug)]
pub struct PieceBlueprint {
    pub move_blueprints: Vec<MoveBlueprint>,
    /// Tests that must all hold for a drop on a square, judged with `TARGET` the square.
    pub drop_conditions: Vec<Test>,
}

impl PieceBlueprint {
    pub fn from_spec(spec: PieceSpec, players_spec: Vec<PlayerSpec>, labels: &mut Labels) -> Self {
        let drop_conditions = spec
            .drop_conditions
            .iter()
            .map(|test| Test::build(test, &players_spec, None, labels))
            .collect();
        PieceBlueprint {
            move_blueprints: spec
                .moves
                .into_iter()
                .map(|x| MoveBlueprint::from_spec(x, players_spec.clone(), labels))
                .collect(),
            drop_conditions,
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

        for move_blueprint in &self.move_blueprints {
            // Each move is a (target position, effect) pair; a later blueprint's move to the
            // same square replaces an earlier one.
            if let Some(value) = move_blueprint.calculate_moves(piece, position, game) {
                moves.extend(value);
            }
        }

        (!moves.is_empty()).then_some(moves)
    }
}
