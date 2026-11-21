mod capture;
pub mod context;
mod convert;
mod copy_source;
mod explode;
mod helpers;
mod move_piece;
mod set_state;

use crate::logic::blueprint::move_blueprint::{SideEffectBlueprint, SideEffectKind};
use crate::logic::Piece;
use crate::shared::BoardChange;

use context::SideEffectContext;

pub fn apply_side_effect(
    side_effect: &SideEffectBlueprint,
    ctx: &SideEffectContext<'_>,
    moved_piece: &mut Piece,
    extra_changes: &mut Vec<BoardChange>,
) {
    match side_effect.kind {
        SideEffectKind::SetState => set_state::apply(side_effect, ctx, moved_piece, extra_changes),
        SideEffectKind::Capture => capture::apply(side_effect, ctx, moved_piece, extra_changes),
        SideEffectKind::MovePiece => {
            move_piece::apply(side_effect, ctx, moved_piece, extra_changes)
        }
        SideEffectKind::Convert => convert::apply(side_effect, ctx, moved_piece, extra_changes),
        SideEffectKind::CopySource => {
            copy_source::apply(side_effect, ctx, moved_piece, extra_changes)
        }
        SideEffectKind::Explode => explode::apply(side_effect, ctx, moved_piece, extra_changes),
        SideEffectKind::Unknown => {}
    }
}
