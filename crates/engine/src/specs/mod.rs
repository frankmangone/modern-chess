pub mod game;

pub use game::parser::parse_spec as parse_game_spec;
pub use game::parser::parse_spec_str as parse_game_spec_str;
pub use game::{
    ActionSpec, BoardShape, BoardSpec, EveryActionSpec, GameSpec, GameSpecError, HexLayout,
    MapSource, MapSpec, MoveSpec, PieceSpec, PlaceKind, PlaceSpec, PlayerSpec, PositionSpec,
    StepSpec,
};

//

pub mod traits;

pub use traits::validate::Validate;
