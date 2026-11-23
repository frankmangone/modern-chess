//! What a game is built with: the rules every copy of a game shares.

use std::collections::HashMap;

use crate::logic::pieces::PieceBlueprint;
use crate::logic::places::holders::HolderInfo;
use crate::logic::places::zones::Zone;
use crate::logic::rules::ends;
use crate::logic::rules::steps::Action;
use crate::logic::rules::test_block::Test;
use crate::shared::{IdMap, Position, Symbol};

/// What a game is built with and never changes while it is played: shared by every copy of
/// the game, so trying a position copies only the state.
#[derive(Debug, Clone)]
pub struct Rules {
    /// The order in which players play, as the spec gives it. The order being played is
    /// `state.turn_order`, which loses players as they are eliminated.
    pub turn_order: Vec<String>,

    // Name of the game, for identification purposes only.
    pub name: String,

    /// The zones declared under the spec's `zones`, by name.
    pub zones: HashMap<String, Zone>,

    // A list of available players. Doubles up as a sort of dynamic enum.
    pub players: Vec<String>,

    // `blueprints` allow for calculation of piece movements.
    pub blueprints: IdMap<Symbol, PieceBlueprint>,

    /// When `true`, captured pieces enter the capturer's hand and can be dropped.
    pub hand_enabled: bool,

    /// Piece code → what it is worth in the material count. Pieces without a value are left out.
    pub piece_values: HashMap<String, u32>,

    /// The holders of the game: each bag (a per-player bag once for each player) and each map
    /// place that holds many, by the name of its place.
    pub holders: IdMap<Position, HolderInfo>,

    /// Maps piece code → the code that enters a hand when the piece is sent there.
    /// `None` means the piece itself (base form) enters the hand.
    pub captured_as: IdMap<Symbol, Option<Symbol>>,

    /// When `true`, a piece that captures may continue jumping (draughts multi-jump).
    pub jump_enabled: bool,

    /// What a player may do on their turn, in spec order.
    pub actions: Vec<Action>,

    /// Whether some test or filter of the spec reads `moved` (castling, a pawn's double step),
    /// so that the position record must tell a piece that has moved from one that has not.
    pub moved_matters: bool,

    /// The end checks, judged in order after every turn (see `GameSpec::end_checks`).
    pub end_checks: Vec<ends::EndCheck>,

    /// The terms of the score, summed for each player (see `GameSpec::score`).
    pub score: Vec<ends::ScoreTerm>,

    /// Tests that narrow the ways, in order (see `GameSpec::must`).
    pub must: Vec<Test>,

    /// Whether some move or action can remove more than its target (a `REMOVE` inside a
    /// `FOR_EACH`, or at `START`), so that an `ATTACKED` test with `keeping` must simulate the capture.
    pub removes_beyond_target: bool,
}
