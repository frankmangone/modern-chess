use crate::logic::PiecesView;
use crate::shared::{IdMap, IdSet};

use crate::logic::game::state::board_changes;
use crate::logic::pieces::PieceBlueprint;
use crate::logic::rules::test_block::{matches, Filter};
use crate::logic::{Board, Game, Piece};
use crate::shared::{BoardChange, Position, Symbol};

impl Game {
    /// Returns the set of positions threatened by all pieces belonging to `attacker`.
    pub fn attacked_by(&self, attacker: &str) -> IdSet<Position> {
        Self::attacked_by_pieces(
            attacker,
            PiecesView::of(&self.state.pieces),
            &self.board,
            &self.rules.blueprints,
        )
    }

    /// Returns the player who acted just before the current turn.
    pub fn previous_player(&self) -> String {
        let len = self.state.turn_order.len();
        let Some(idx) = (self.state.current_turn as usize + len.saturating_sub(1)).checked_rem(len)
        else {
            return String::new();
        };
        self.state.turn_order.get(idx).cloned().unwrap_or_default()
    }

    /// Computes the attack set for `attacker` using an explicit pieces map (for simulation).
    pub(crate) fn attacked_by_pieces(
        attacker: &str,
        pieces: PiecesView<'_>,
        board: &Board,
        blueprints: &IdMap<Symbol, PieceBlueprint>,
    ) -> IdSet<Position> {
        pieces
            .iter()
            .filter(|(_, p)| p.player == attacker)
            .filter_map(|(pos, p)| blueprints.get(&p.code).map(|bp| (pos, bp)))
            .flat_map(|(pos, bp)| bp.calculate_threats_with(attacker, pos, pieces, board))
            .collect()
    }

    /// Whether some piece of `attacker` has a threat on one of `places` that
    /// `capture_is_a_real_threat` accepts, given `keeping`. It stops at the first such piece and
    /// never builds the set of everything `attacker` threatens.
    pub(crate) fn reaches(
        &self,
        attacker: &str,
        pieces: PiecesView<'_>,
        places: &[&Position],
        keeping: Option<&Filter>,
    ) -> bool {
        if places.is_empty() {
            return false;
        }
        pieces
            .iter()
            .filter(|(_, piece)| piece.player == attacker)
            .any(|(from, piece)| {
                let Some(bp) = self.rules.blueprints.get(&piece.code) else {
                    return false;
                };
                // A target found not to be a real threat is not simulated again when another
                // move of the same piece reaches it.
                let mut refused: Vec<&Position> = Vec::new();
                bp.visit_threats(attacker, from, pieces, &self.board, &mut |threat| {
                    places.iter().any(|target| {
                        if *target != threat || refused.contains(target) {
                            return false;
                        }
                        let real =
                            self.capture_is_a_real_threat(from, piece, target, pieces, keeping);
                        if !real {
                            refused.push(target);
                        }
                        real
                    })
                })
            })
    }

    /// Whether `attacker` threatens one of `places` within two of its moves in a row, with no
    /// reply between them: it already does (as `reaches`), or some piece moves to a square it
    /// threatens, taking what stands there, and then some piece of it threatens one on the
    /// position that leaves. Only moves that threaten are first moves, and only their change
    /// of place is applied (no promotion, push or castling), so it is exact for pieces whose
    /// moves are their captures. A move's conditions are ignored, as `reaches` ignores them.
    pub(crate) fn reaches_in_two(
        &self,
        attacker: &str,
        pieces: PiecesView<'_>,
        places: &[&Position],
    ) -> bool {
        if self.reaches(attacker, pieces, places, None) {
            return true;
        }
        if places.is_empty() {
            return false;
        }
        let base = pieces.to_map();
        base.iter()
            .filter(|(_, piece)| piece.player == attacker)
            .any(|(from, piece)| {
                let Some(bp) = self.rules.blueprints.get(&piece.code) else {
                    return false;
                };
                bp.visit_threats(
                    attacker,
                    from,
                    PiecesView::of(&base),
                    &self.board,
                    &mut |to| {
                        let changes = [
                            BoardChange::clear(from),
                            BoardChange::set_piece(*to, piece.clone()),
                        ];
                        self.reaches(attacker, PiecesView::with(&base, &changes), places, None)
                    },
                )
            })
    }

    /// Whether the piece at `from` could really take the piece at `target`.
    ///
    /// It always can, unless some move removes more than its target (captures explode) and the
    /// test names a piece to keep (`keeping`): then a capture that would leave the attacker
    /// without a piece it had, such as its king next to the one it takes, is no threat. This is
    /// why two kings may stand side by side in Atomic Chess. Only those games pay for the
    /// simulation, which copies the game.
    pub(crate) fn capture_is_a_real_threat(
        &self,
        from: &Position,
        attacker: &Piece,
        target: &Position,
        pieces: PiecesView<'_>,
        keeping: Option<&Filter>,
    ) -> bool {
        let Some(keeping) = keeping else {
            return true;
        };
        let owns_one = |pieces: PiecesView<'_>| {
            pieces
                .values()
                .any(|p| p.player == attacker.player && matches(keeping, p, &attacker.player))
        };
        if !self.rules.removes_beyond_target || !owns_one(pieces) {
            return true;
        }
        // The moves are worked out on the simulated board with the attacker to move, so the
        // blast is too. Without that, the attacked piece would count as the mover's own.
        let Some(turn) = self
            .state
            .turn_order
            .iter()
            .position(|p| *p == attacker.player)
        else {
            return true;
        };
        let mut simulated = self.clone();
        simulated.state.pieces = pieces.to_map();
        simulated.state.current_turn = turn as u8;
        simulated.state.asked.clear();
        simulated.state.focus = None;
        simulated.state.together = None;
        // The capture as the attacker's moves make it: the first way to `target` in spec order.
        let changes = simulated
            .piece_move_ways(from, target)
            .into_iter()
            .next()
            .map(|way| way.changes);
        let Some(changes) = changes else {
            return true;
        };
        let after = board_changes::simulate_from(&simulated.state.pieces, &changes);
        owns_one(PiecesView::of(&after))
    }
}
