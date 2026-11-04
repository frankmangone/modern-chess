pub mod game;

pub use game::parser::parse_spec as parse_game_spec;
pub use game::parser::parse_spec_str as parse_game_spec_str;
pub use game::{
    ActionSpec, BoardSpec, ConditionSpec, GameSpec, GameSpecError, MoveSpec, PieceSpec, PlayerSpec,
    WinConditionSpec,
};

//

pub mod traits;

pub use traits::validate::Validate;
