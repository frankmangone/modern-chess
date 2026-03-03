mod capture;
pub mod context;
mod convert;
mod copy_source;
mod explode;
mod helpers;
mod move_piece;
mod set_state;

use crate::logic::blueprint::move_blueprint::SideEffectBlueprint;
use crate::logic::Piece;
use crate::shared::{BoardChange, CAPTURE, CONVERT, COPY_SOURCE, EXPLODE, MOVE, SET_STATE};

use context::SideEffectContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SideEffectAction {
    SetState,
    Capture,
    MovePiece,
    Convert,
    CopySource,
    Explode,
    Unknown,
}

impl From<&str> for SideEffectAction {
    fn from(value: &str) -> Self {
        match value {
            SET_STATE => Self::SetState,
            CAPTURE => Self::Capture,
            MOVE => Self::MovePiece,
            CONVERT => Self::Convert,
            COPY_SOURCE => Self::CopySource,
            EXPLODE => Self::Explode,
            _ => Self::Unknown,
        }
    }
}

pub fn apply_side_effect(
    side_effect: &SideEffectBlueprint,
    ctx: &SideEffectContext<'_>,
    moved_piece: &mut Piece,
    extra_changes: &mut Vec<BoardChange>,
) {
    match SideEffectAction::from(side_effect.action.as_str()) {
        SideEffectAction::SetState => {
            set_state::apply(side_effect, ctx, moved_piece, extra_changes)
        }
        SideEffectAction::Capture => capture::apply(side_effect, ctx, moved_piece, extra_changes),
        SideEffectAction::MovePiece => {
            move_piece::apply(side_effect, ctx, moved_piece, extra_changes)
        }
        SideEffectAction::Convert => convert::apply(side_effect, ctx, moved_piece, extra_changes),
        SideEffectAction::CopySource => {
            copy_source::apply(side_effect, ctx, moved_piece, extra_changes)
        }
        SideEffectAction::Explode => explode::apply(side_effect, ctx, moved_piece, extra_changes),
        SideEffectAction::Unknown => {}
    }
}
