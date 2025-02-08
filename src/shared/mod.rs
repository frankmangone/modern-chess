mod position;
mod effect;
mod constants;

pub use constants::{
    EMPTY,
    NOT_EMPTY,
    ENEMY,
    ALLY,
    //
    MOVE,
    CAPTURE,
    //
    FIRST_MOVE,
    NOT_ATTACKED,
    DEPENDS_ON,
    //
    POSITION,
    STATE,
};

pub use position::{
    Position,
    into_position,
    ExtendedPosition,
    into_extended_position,
    into_string,
};

pub use effect::{
    Effect, 
    BoardChange,
};
