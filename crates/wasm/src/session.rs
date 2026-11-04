use std::collections::BTreeMap;

use modern_chess::logic::{Game, GamePhase};
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
