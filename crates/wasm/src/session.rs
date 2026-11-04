use std::collections::BTreeMap;

use modern_chess::logic::{Game, GamePhase, GameTransition};
use modern_chess::shared::POSITION;
use modern_chess::specs::parse_game_spec_str;

use crate::view::{PieceView, Square, Status, View};

/// A playable game: the engine plus the text of the spec it was built from.
pub struct Session {
    game: Game,
    spec_json: String,
}

fn build_game(spec_json: &str) -> Result<Game, String> {
    let spec = parse_game_spec_str(spec_json).map_err(|e| format!("{e:?}"))?;
    Ok(Game::from_spec(spec))
}

fn square(position: &[u8]) -> Option<Square> {
    match position {
        [x, y] => Some([*x, *y]),
        _ => None,
    }
}

impl Session {
    pub fn new(spec_json: &str) -> Result<Session, String> {
        Ok(Session {
            game: build_game(spec_json)?,
            spec_json: spec_json.to_string(),
        })
    }

    pub fn reset(&mut self) {
        // The text was validated in `new`, so this cannot fail.
        if let Ok(game) = build_game(&self.spec_json) {
            self.game = game;
        }
    }

    fn ensure_playable(&self) -> Result<(), String> {
        if self.game.hand_enabled {
            return Err("drops are not supported yet".to_string());
        }
        if matches!(self.game.state.phase, GamePhase::GameOver { .. }) {
            return Err("the game is over".to_string());
        }
        Ok(())
    }

    /// Selects the piece at `(x, y)` and computes its legal destinations.
    /// On error the game is left exactly as it was.
    pub fn select(&mut self, x: u8, y: u8) -> Result<(), String> {
        self.ensure_playable()?;
        if matches!(self.game.state.phase, GamePhase::Transforming { .. }) {
            return Err("choose a promotion first".to_string());
        }
        // Mid-jump, only the jumping piece may be picked. The engine would refuse with
        // a bare `InvalidPlayer`, which means nothing to a reader.
        if let GamePhase::Jumping { position } = &self.game.state.phase {
            if position.as_slice() != [x, y] {
                return Err("finish the jump with the selected piece first".to_string());
            }
        }
        self.game
            .transition(GameTransition::CalculateMoves {
                position: vec![x, y],
            })
            .map_err(|e| format!("{e:?}"))
    }

    /// Plays the selected piece to `(x, y)`. The engine advances the turn.
    /// The destination is checked here first: the engine resets to idle on an
    /// invalid destination, which would drop a jump in progress.
    pub fn make_move(&mut self, x: u8, y: u8) -> Result<(), String> {
        self.ensure_playable()?;
        let position = vec![x, y];
        let is_destination = self
            .game
            .state
            .available_moves
            .as_ref()
            .is_some_and(|moves| moves.contains_key(&position));
        if !is_destination {
            return Err("not a legal destination".to_string());
        }
        self.game
            .transition(GameTransition::ExecuteMove { position })
            .map_err(|e| format!("{e:?}"))
    }

    /// Answers a pending promotion choice.
    pub fn promote(&mut self, code: &str) -> Result<(), String> {
        self.game
            .transition(GameTransition::Transform {
                target: code.to_string(),
            })
            .map_err(|e| format!("{e:?}"))
    }

    pub fn view(&self) -> View {
        let game = &self.game;

        let mut disabled: Vec<Square> = game
            .board
            .disabled_positions
            .iter()
            .filter_map(|p| square(p))
            .collect();
        disabled.sort();

        let mut pieces: Vec<PieceView> = game
            .state
            .pieces
            .iter()
            .filter_map(|(position, piece)| {
                square(position).map(|[x, y]| PieceView {
                    x,
                    y,
                    code: piece.code.clone(),
                    player: piece.player.clone(),
                })
            })
            .collect();
        pieces.sort_by_key(|p| (p.y, p.x));

        let mut selected = None;
        let mut promotion = None;
        let mut status = Status::Playing;
        match &game.state.phase {
            GamePhase::Moving { position } | GamePhase::Jumping { position } => {
                selected = square(position);
            }
            GamePhase::Transforming { options, .. } => promotion = Some(options.clone()),
            GamePhase::GameOver { winner } => {
                status = match winner {
                    Some(winner) => Status::Won {
                        winner: winner.clone(),
                    },
                    None => Status::Draw,
                }
            }
            GamePhase::Idle | GamePhase::Dropping { .. } => {}
        }

        let mut destinations: Vec<Square> = match (&selected, &game.state.available_moves) {
            (Some(_), Some(moves)) => moves.keys().filter_map(|p| square(p)).collect(),
            _ => Vec::new(),
        };
        destinations.sort();

        let mut zones: BTreeMap<String, BTreeMap<String, Vec<Square>>> = BTreeMap::new();
        for (name, condition) in &game.conditions {
            if condition.r#type != POSITION {
                continue;
            }
            let per_player = condition
                .check
                .iter()
                .map(|(player, squares)| {
                    let mut parsed: Vec<Square> = squares
                        .iter()
                        .filter_map(|s| {
                            let mut parts = s.split(',').map(|n| n.trim().parse::<u8>());
                            match (parts.next(), parts.next(), parts.next()) {
                                (Some(Ok(x)), Some(Ok(y)), None) => Some([x, y]),
                                _ => None,
                            }
                        })
                        .collect();
                    parsed.sort();
                    (player.clone(), parsed)
                })
                .collect();
            zones.insert(name.clone(), per_player);
        }

        View {
            width: game.board.dimensions.first().copied().unwrap_or(0),
            height: game.board.dimensions.get(1).copied().unwrap_or(0),
            disabled,
            players: game.players.clone(),
            pieces,
            turn: game.current_player(),
            selected,
            destinations,
            promotion,
            status,
            zones,
            hands: game.hand_enabled,
        }
    }
}
