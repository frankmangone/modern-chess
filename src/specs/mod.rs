pub mod game_specs;

pub use game_specs::structs::game::{GameSpec, GameSpecError};
pub use game_specs::parser::parse_spec as parse_game_spec;

//

pub mod piece_specs;

pub use piece_specs::structs::piece::PieceSpec;
pub use piece_specs::parser::parse_spec as parse_piece_spec;
