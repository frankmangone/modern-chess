//! An `ASK` that asks all at once (`TOGETHER`): every asked seat owes an answer, in any order.
//!
//! While it lasts nobody is the player to move. A seat answers by being **focused**: for the length
//! of one answer the game is played as that seat (it is the player to move, in the stage it is
//! asked in, with its own answers so far), so the choices, ways and tests are the ones any turn has.
//! When its answer is whole, a sealed ask keeps it (the action and the answers that find the way
//! again) and an open ask plays it at once. A sealed ask plays every answer, in the order the seats
//! were asked, once the last seat has answered; each way is worked out on the position the seats
//! were asked on, so no seat's choices depend on how quickly the others answer.

use super::choose::stage_pin;
use crate::logic::game::state::{LockedWay, StageState, Together, TogetherSeat};
use crate::logic::rules::steps::AskStep;
use crate::logic::{Answer, Game, GameError, GamePhase, Way};

impl Game {
    /// Whether an `ASK` that asks all at once is waiting on seats.
    pub fn together_pending(&self) -> bool {
        self.state.together.is_some()
    }

    /// The seats that still owe an answer to an `ASK` that asks all at once, in the order asked.
    /// A sealed answer given is not owed any more.
    pub fn waiting(&self) -> Vec<String> {
        self.state
            .together
            .iter()
            .flat_map(|together| &together.seats)
            .filter(|seat| seat.locked.is_none())
            .map(|seat| seat.player.clone())
            .collect()
    }

    /// Whether `player` owes an answer to an `ASK` that asks all at once.
    pub fn owes_answer(&self, player: &str) -> bool {
        self.waiting().iter().any(|seat| seat == player)
    }

    /// Plays the game as the seat `player` owes an answer for, until `unfocus`: the choices,
    /// answers and cancelling are then the seat's. Refused for a seat that owes nothing.
    pub fn focus_seat(&mut self, player: &str) -> Result<(), GameError> {
        let Some(together) = &self.state.together else {
            return Err(GameError::NotAnOption);
        };
        let Some(seat) = together
            .seats
            .iter()
            .find(|seat| seat.player == player && seat.locked.is_none())
        else {
            return Err(GameError::NotAnOption);
        };
        let chosen = seat.chosen.clone();
        self.state.stage = Some(StageState {
            name: together.stage,
            last: None,
        });
        self.state.focus = Some(player.to_string());
        self.state.phase = GamePhase::Choosing { chosen };
        self.work_out_ways();
        // As on entering any stage: when every way starts with the same piece it is chosen already.
        if let GamePhase::Choosing { chosen } = &mut self.state.phase {
            if chosen.is_empty() {
                let pinned = self.state.ways.as_deref().and_then(stage_pin);
                chosen.extend(pinned.map(Answer::Place));
            }
        }
        Ok(())
    }

    /// Stops playing as a seat, keeping the answers it has given so far. Does nothing when the
    /// game is not being played as a seat (a seat's answer that was whole already did it).
    pub fn unfocus(&mut self) {
        let Some(player) = self.state.focus.take() else {
            return;
        };
        let chosen = match &self.state.phase {
            GamePhase::Choosing { chosen } => chosen.clone(),
            GamePhase::GameOver { .. } => Vec::new(),
        };
        if let Some(seat) = self.seat_mut(&player) {
            if seat.locked.is_none() {
                seat.chosen = chosen;
            }
        }
        self.idle_together();
    }

    /// Answers the next choice of the seat `player`, as `choose` does for the player to move.
    pub fn choose_as(&mut self, player: &str, answer: Answer) -> Result<(), GameError> {
        self.focus_seat(player)?;
        let result = self.choose(answer);
        self.unfocus();
        result
    }

    /// Takes back the answers the seat `player` has given so far, as `cancel` does.
    pub fn cancel_as(&mut self, player: &str) -> Result<(), GameError> {
        self.focus_seat(player)?;
        let result = self.cancel();
        self.unfocus();
        result
    }

    /// A copy of the game played as the seat `player`, to read its choice from; `None` when it owes
    /// no answer.
    pub fn focused(&self, player: &str) -> Option<Game> {
        let mut game = self.clone();
        game.focus_seat(player).ok()?;
        Some(game)
    }

    /// A copy of the game with the kept answer of the seat `player` played on it, to show that seat
    /// what it answered while the others have not yet: the card it bid is out of its hand and in
    /// its bid. Only that seat may be shown it, since the answer is a secret until every seat is
    /// in. `None` when the seat has no kept answer (an open ask keeps none).
    pub fn with_kept_answer(&self, player: &str) -> Option<Game> {
        let locked = self
            .state
            .together
            .as_ref()?
            .seats
            .iter()
            .find(|seat| seat.player == player)?
            .locked
            .clone()?;
        let mut game = self.clone();
        let way = game
            .seat_ways(player)
            .into_iter()
            .find(|way| way.action == locked.action && way.answers == locked.answers)?;
        game.apply_way(&way, player);
        Some(game)
    }

    /// The state nobody is to move in, as it stands between answers.
    fn idle_together(&mut self) {
        self.state.focus = None;
        self.state.stage = None;
        self.state.phase = GamePhase::Choosing { chosen: Vec::new() };
        self.state.ways = Some(Vec::new());
    }

    fn seat_mut(&mut self, player: &str) -> Option<&mut TogetherSeat> {
        self.state
            .together
            .as_mut()?
            .seats
            .iter_mut()
            .find(|seat| seat.player == player)
    }

    /// The ways `player` has in the stage of the ask, on the position as it stands.
    pub(crate) fn seat_ways(&mut self, player: &str) -> Vec<Way> {
        let Some(stage) = self.state.together.as_ref().map(|together| together.stage) else {
            return Vec::new();
        };
        let before = (self.state.focus.take(), self.state.stage.take());
        self.state.focus = Some(player.to_string());
        self.state.stage = Some(StageState {
            name: stage,
            last: None,
        });
        let ways = self.compute_ways();
        (self.state.focus, self.state.stage) = before;
        ways
    }

    /// Asks `names` all at once, those with a way in the stage. False, and nothing asked, when none
    /// has one.
    pub(super) fn start_together(&mut self, ask: &AskStep, names: Vec<String>) -> bool {
        self.state.together = Some(Together {
            stage: ask.stage,
            sealed: ask.sealed,
            then: ask.then,
            seats: names
                .into_iter()
                .map(|player| TogetherSeat {
                    player,
                    chosen: Vec::new(),
                    locked: None,
                })
                .collect(),
        });
        self.drop_seats_without_ways();
        if self.waiting().is_empty() {
            self.state.together = None;
            return false;
        }
        self.idle_together();
        true
    }

    /// Takes out the seats that owe an answer and have no way left to give it.
    fn drop_seats_without_ways(&mut self) {
        let owing = self.waiting();
        let stuck: Vec<String> = owing
            .into_iter()
            .filter(|player| self.seat_ways(player).is_empty())
            .collect();
        if let Some(together) = self.state.together.as_mut() {
            together.seats.retain(|seat| !stuck.contains(&seat.player));
        }
    }

    /// A seat's way is whole. A sealed ask keeps it until every seat has answered; an open one
    /// plays it now, and every other seat starts its answer again on the position it leaves.
    pub(super) fn play_together_way(&mut self, way: Way) {
        let Some(player) = self.state.focus.take() else {
            return;
        };
        self.idle_together();
        let sealed = self.state.together.as_ref().is_some_and(|t| t.sealed);
        if sealed {
            if let Some(seat) = self.seat_mut(&player) {
                seat.chosen.clear();
                seat.locked = Some(LockedWay {
                    action: way.action,
                    answers: way.answers,
                });
            }
            if self.waiting().is_empty() {
                self.reveal_together();
            }
            return;
        }
        if let Some(together) = self.state.together.as_mut() {
            together.seats.retain(|seat| seat.player != player);
            for seat in &mut together.seats {
                seat.chosen.clear();
            }
        }
        self.apply_way(&way, &player);
        self.idle_together();
        self.drop_seats_without_ways();
        if self.waiting().is_empty() {
            self.finish_together();
        }
    }

    /// Every seat of a sealed ask has answered: plays the answers in the order the seats were
    /// asked, each as it was worked out on the position they were all asked on.
    fn reveal_together(&mut self) {
        let Some(seats) = self.state.together.as_ref().map(|t| t.seats.clone()) else {
            return;
        };
        let mut plays: Vec<(String, Way)> = Vec::new();
        for seat in &seats {
            let Some(locked) = &seat.locked else {
                continue;
            };
            let found = self
                .seat_ways(&seat.player)
                .into_iter()
                .find(|way| way.action == locked.action && way.answers == locked.answers);
            // A kept answer found its way when it was given, and nothing is played between that and
            // here; a restored state is checked for it (`stage_fits_the_rules`).
            if let Some(way) = found {
                plays.push((seat.player.clone(), way));
            }
        }
        for (player, way) in &plays {
            self.apply_way(way, player);
        }
        self.finish_together();
    }

    /// The seats have all answered: the asker moves again in the `then` stage, if it has a way
    /// there, and otherwise the turn is handed on.
    fn finish_together(&mut self) {
        let then = self
            .state
            .together
            .take()
            .and_then(|together| together.then);
        self.state.focus = None;
        self.state.stage = None;
        if let Some(then) = then {
            if self.enter_stage(then, None) {
                return;
            }
        }
        self.next_turn();
        self.check_game_over();
    }
}
