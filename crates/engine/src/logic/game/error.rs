#[derive(Debug)]
pub enum GameError {
    /// The game is over.
    InvalidGamePhase,

    /// The answer leads to no way of taking an action.
    NotAnOption,

    /// The answers cannot be taken back: a jump is under way, or what they do is showing.
    CannotCancel,
}
