use crate::logic::Answer;

#[derive(Debug, Clone)]
pub enum GameTransition {
    /// Answers the next choice of the turn: a place, a code, a piece in the hand, or an
    /// action's name. Once the answers match exactly one complete way, it is played.
    Choose(Answer),

    /// Takes back the answers given so far this turn.
    Cancel,
}
