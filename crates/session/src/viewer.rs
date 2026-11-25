use modern_chess::specs::Visible;

/// Who is looking at a game: what a view may show depends on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Viewer {
    /// A seat: the player of that name. Sees the bags that everyone sees, and its own where the
    /// bag says its owner does.
    Player(String),
    /// Someone watching, who is no player: sees what everyone sees.
    Spectator,
    /// Nobody in particular: one person playing every side (the "Try it" board, the book, the
    /// tests). Sees everything.
    Everyone,
}

impl Viewer {
    /// Whether this viewer sees what a bag with `visible` keeps, `owner` being the player it
    /// belongs to (`None` for a shared bag). Says nothing of the end of the game, when
    /// everything is shown.
    pub fn sees(&self, visible: Visible, owner: Option<&str>) -> bool {
        match (self, visible) {
            (_, Visible::All) | (Viewer::Everyone, _) => true,
            (Viewer::Player(name), Visible::Owner) => owner == Some(name.as_str()),
            (Viewer::Player(_) | Viewer::Spectator, _) => false,
        }
    }

    /// Whether this viewer is the player to move, who is the only one with a choice to make.
    /// `Everyone` is, as a hot seat plays every side.
    pub fn is_mover(&self, mover: &str) -> bool {
        match self {
            Viewer::Everyone => true,
            Viewer::Player(name) => name == mover,
            Viewer::Spectator => false,
        }
    }
}
