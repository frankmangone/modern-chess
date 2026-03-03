pub mod game;

pub use game::parser::parse_spec as parse_game_spec;
pub use game::{
    ActionSpec, BoardSpec, ConditionSpec, GameSpec, GameSpecError, MoveSpec, PieceSpec, PlayerSpec,
    WinConditionSpec,
};

//

pub mod traits;

pub use traits::validate::Validate;
