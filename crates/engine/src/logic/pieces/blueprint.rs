use crate::logic::PiecesView;
use crate::shared::IdSet;

use crate::logic::{Game, Labels};
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

    /// Returns all squares this piece attacks from `position`: those one of its moves could take
    /// on (see `MoveBlueprint::visit_threats`).
    pub fn calculate_threats_with(
        &self,
        game: &Game,
        player: &str,
        position: &Position,
        pieces: PiecesView<'_>,
    ) -> IdSet<Position> {
        self.move_blueprints
            .iter()
            .flat_map(|mb| mb.calculate_threats_with(game, player, position, pieces))
            .collect()
    }

    /// Calls `found` with each square this piece attacks from `position`, and stops at the first
    /// for which it returns true, which is then the result. No set is built. With `wanted`, only
    /// those squares are judged.
    pub(crate) fn visit_threats(
        &self,
        game: &Game,
        player: &str,
        position: &Position,
        pieces: PiecesView<'_>,
        wanted: Option<&[&Position]>,
        found: &mut impl FnMut(&Position) -> bool,
    ) -> bool {
        self.move_blueprints
            .iter()
            .any(|mb| mb.visit_threats(game, player, position, pieces, wanted, found))
    }
}
