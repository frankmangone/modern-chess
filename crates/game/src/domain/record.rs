use serde::Serialize;

use super::{Outcome, SeatName};

/// How a finished match went for one seat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeatResult {
    Won,
    Drawn,
    Lost,
}

/// How `outcome` went for `seat`. This is the only place that turns an outcome into a result, so
/// it is the only place to change if a game can ever have several winners: today an outcome is
/// one winning seat or a draw, and everyone else lost.
pub fn result_for(outcome: &Outcome, seat: &SeatName) -> SeatResult {
    match outcome {
        Outcome::Won { seat: winner, .. } if winner == seat => SeatResult::Won,
        Outcome::Won { .. } => SeatResult::Lost,
        Outcome::Draw => SeatResult::Drawn,
    }
}

/// What a person has done on a game: matches won, drawn and lost.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Record {
    pub won: u32,
    pub drawn: u32,
    pub lost: u32,
}

impl Record {
    pub fn add(&mut self, result: SeatResult) {
        match result {
            SeatResult::Won => self.won += 1,
            SeatResult::Drawn => self.drawn += 1,
            SeatResult::Lost => self.lost += 1,
        }
    }

    pub fn played(&self) -> u32 {
        self.won + self.drawn + self.lost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WinReason;

    fn seat(name: &str) -> SeatName {
        SeatName::new(name)
    }

    #[test]
    fn the_winner_wins_and_every_other_seat_loses() {
        let outcome = Outcome::Won {
            seat: seat("WHITE"),
            reason: None,
        };

        assert_eq!(result_for(&outcome, &seat("WHITE")), SeatResult::Won);
        assert_eq!(result_for(&outcome, &seat("BLACK")), SeatResult::Lost);
        assert_eq!(result_for(&outcome, &seat("RED")), SeatResult::Lost);
    }

    #[test]
    fn giving_up_is_a_loss_for_the_one_who_gave_up() {
        let outcome = Outcome::Won {
            seat: seat("BLACK"),
            reason: Some(WinReason::Resigned),
        };

        assert_eq!(result_for(&outcome, &seat("WHITE")), SeatResult::Lost);
        assert_eq!(result_for(&outcome, &seat("BLACK")), SeatResult::Won);
    }

    #[test]
    fn a_draw_is_a_draw_for_everyone() {
        for name in ["WHITE", "BLACK"] {
            assert_eq!(result_for(&Outcome::Draw, &seat(name)), SeatResult::Drawn);
        }
    }

    #[test]
    fn a_record_counts_each_result_and_the_matches_played() {
        let mut record = Record::default();
        for result in [
            SeatResult::Won,
            SeatResult::Won,
            SeatResult::Drawn,
            SeatResult::Lost,
        ] {
            record.add(result);
        }

        assert_eq!(
            record,
            Record {
                won: 2,
                drawn: 1,
                lost: 1
            }
        );
        assert_eq!(record.played(), 4);
    }
}
