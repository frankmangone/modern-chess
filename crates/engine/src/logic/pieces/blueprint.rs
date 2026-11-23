use crate::logic::PiecesView;
use crate::shared::IdSet;

use crate::logic::{Board, Game, Labels};
use crate::shared::Position;
use crate::specs::{PieceSpec, PlayerSpec};

use super::moves::MoveBlueprint;

/// A `PieceBlueprint` is essentially a factory for piece threat calculation and the steps of
/// its moves. It stores the set of rules, but without knowledge
/// of the actual piece position.
///
/// A piece blueprint is associated with a piece code, and contains a list of move blueprints.
/// Each move blueprint is a factory for a single move.
#[derive(Clone, Debug)]
pub struct PieceBlueprint {
    pub move_blueprints: Vec<MoveBlueprint>,
}

impl PieceBlueprint {
    pub fn from_spec(spec: PieceSpec, players_spec: Vec<PlayerSpec>, labels: &mut Labels) -> Self {
        PieceBlueprint {
            move_blueprints: spec
                .moves
                .into_iter()
                .map(|x| MoveBlueprint::from_spec(x, players_spec.clone(), labels))
                .collect(),
        }
    }

    /// Returns all squares threatened by this piece from `position`.
    pub fn calculate_threats(
        &self,
        player: &str,
        position: &Position,
        game: &Game,
    ) -> IdSet<Position> {
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
        pieces: PiecesView<'_>,
        board: &Board,
    ) -> IdSet<Position> {
        self.move_blueprints
            .iter()
            .flat_map(|mb| mb.calculate_threats_with(player, position, pieces, board))
            .collect()
    }

    /// Calls `found` with each square this piece threatens from `position`, and stops at the
    /// first for which it returns true, which is then the result. No set is built.
    pub(crate) fn visit_threats(
        &self,
        player: &str,
        position: &Position,
        pieces: PiecesView<'_>,
        board: &Board,
        found: &mut impl FnMut(&Position) -> bool,
    ) -> bool {
        self.move_blueprints
            .iter()
            .any(|mb| mb.visit_threats(player, position, pieces, board, found))
    }
}
