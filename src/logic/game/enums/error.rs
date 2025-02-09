#[derive(Debug)]
pub enum GameError {
    // General
    InvalidGamePhase,
    NoAvailableMoves,

    // Move calculation errors
    InvalidPlayer,
    NoPieceInPosition,
    
    // Move execution errors
    InvalidMove,

    // Transformation errors
    InvalidTransformationOption,
}
