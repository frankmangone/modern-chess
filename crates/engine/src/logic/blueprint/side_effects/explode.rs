use crate::logic::blueprint::move_blueprint::SideEffectBlueprint;
use crate::logic::Piece;
use crate::shared::BoardChange;

use super::context::SideEffectContext;

/// Atomic-chess explosion: clears all pieces within `radius` squares of the landing square,
/// along every axis of the board.
///
/// - The center square (landing square) is **always** cleared, regardless of `exempt_codes`.
/// - Adjacent squares within the radius are cleared unless the piece's code is in `exempt_codes`.
/// - Off-board squares are silently skipped.
/// - Empty squares produce no extra change (already handled by the primary move's `clear(source)`
///   + `set_piece(target)` board changes, but we emit a redundant clear for simplicity — it is
///   idempotent).
pub fn apply(
    side_effect: &SideEffectBlueprint,
    ctx: &SideEffectContext<'_>,
    _moved_piece: &mut Piece,
    extra_changes: &mut Vec<BoardChange>,
) {
    let radius = side_effect.radius.unwrap_or(1);
    let exempt = &side_effect.exempt_codes;
    for position in ctx.game.board.positions_within(ctx.target_position, radius) {
        if position == *ctx.target_position {
            // Center always cleared — the capturing/exploding piece is destroyed too.
            extra_changes.push(BoardChange::clear(&position));
        } else if let Some(piece) = ctx.game.piece_at_position_ref(&position) {
            // Adjacent square: clear unless the piece code is exempt.
            if !exempt.contains(&piece.code) {
                extra_changes.push(BoardChange::clear(&position));
            }
        }
    }
}
