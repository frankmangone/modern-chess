pub mod game;

pub use game::{
    GameSpec,
    GameSpecError,
    BoardSpec,
    PlayerSpec,
    TurnSpec,
};
pub use game::parser::parse_spec as parse_game_spec;

//

pub mod traits;

pub use traits::validate::Validate;