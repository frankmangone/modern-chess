mod position;
mod effect;
mod states;

pub use states::{
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
};

pub use position::{
    Position,
    into_position,
    ExtendedPosition,
    into_extended_position,
};

pub use effect::{
    Effect, 
    BoardChange,
};
