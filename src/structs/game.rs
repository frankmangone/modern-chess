use crate::specs::GameSpec;

#[derive(Debug)]
pub struct Game {
    name: String,
}

impl Game {
    pub fn from_spec(spec: GameSpec) -> Game {
        Game {
            name: spec.name
        }
    }
}