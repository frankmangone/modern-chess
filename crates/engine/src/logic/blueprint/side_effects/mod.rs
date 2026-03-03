mod capture;
pub mod context;
mod convert;
mod copy_source;
mod helpers;
mod move_piece;
mod set_state;

use crate::logic::blueprint::move_blueprint::SideEffectBlueprint;
use crate::logic::Piece;
use crate::shared::{BoardChange, CAPTURE, CONVERT, COPY_SOURCE, MOVE, SET_STATE};

use context::SideEffectContext;

pub fn apply_side_effect(
    side_effect: &SideEffectBlueprint,
    ctx: &SideEffectContext<'_>,
    moved_piece: &mut Piece,
    extra_changes: &mut Vec<BoardChange>,
) {
    match side_effect.action.as_str() {
        SET_STATE => set_state::apply(side_effect, ctx, moved_piece, extra_changes),
        CAPTURE => capture::apply(side_effect, ctx, moved_piece, extra_changes),
        MOVE => move_piece::apply(side_effect, ctx, moved_piece, extra_changes),
        CONVERT => convert::apply(side_effect, ctx, moved_piece, extra_changes),
        COPY_SOURCE => copy_source::apply(side_effect, ctx, moved_piece, extra_changes),
        _ => {}
    }
}
