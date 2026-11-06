use std::collections::HashMap;

use crate::logic::{Game, GamePhase};
use crate::shared::{Effect, Position};

impl Game {
    pub(crate) fn enter_moving_phase(
        &mut self,
        position: Position,
        moves: HashMap<Position, Effect>,
    ) {
        self.state.available_moves = Some(moves);
        self.state.phase = GamePhase::Moving {
            position,
            locked: false,
            capture_only: false,
        };
    }

    /// Enter a locked jump-continuation: the player must keep jumping with the same piece.
    pub(crate) fn enter_jump_continuation(
        &mut self,
        position: Position,
        moves: HashMap<Position, Effect>,
    ) {
        self.state.available_moves = Some(moves);
        self.state.phase = GamePhase::Moving {
            position,
            locked: true,
            capture_only: true,
        };
    }

    pub(crate) fn enter_dropping_phase(
        &mut self,
        piece_code: String,
        moves: HashMap<Position, Effect>,
    ) {
        self.state.available_moves = Some(moves);
        self.state.phase = GamePhase::Dropping { piece_code };
    }

    pub(crate) fn enter_transforming_phase(&mut self, position: Position, options: Vec<String>) {
        self.state.available_moves = None;
        self.state.phase = GamePhase::Transforming { position, options };
    }

    pub(crate) fn set_idle_state(&mut self) {
        self.state.available_moves = None;
        self.state.phase = GamePhase::Idle;
    }
}
