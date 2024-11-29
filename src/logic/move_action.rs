pub enum Action {
    None,
    Move,
    Take,
    Custom(String)
}

struct MoveAction {
    state: String,
    action: Action,
}

impl MoveAction {
    pub fn from_spec(spec: ActionSpec) -> Self {
        MoveAction {
            state: spec.action,
            action: 
            // TODO: Parse the rest of the spec
        }
    }
}