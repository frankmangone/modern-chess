use std::collections::BTreeMap;

use serde_json::Value;

use modern_chess::logic::{Game, GamePhase, GameTransition};
use modern_chess::shared::POSITION;
use modern_chess::specs::parse_game_spec_str;

use crate::view::{PieceView, Square, Status, View};

/// A playable game: the engine plus the text of the spec it was built from.
pub struct Session {
    game: Game,
    spec_json: String,
    revision: u64,
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
            revision: 0,
        })
    }

    /// Rebuilds a session from the rules and a `state()` saved earlier, as the game server does
    /// on every request. A piece that was picked up stays picked up: its moves are recomputed,
    /// since the saved state does not carry them.
    pub fn resume(spec_json: &str, state: &Value, revision: u64) -> Result<Session, String> {
        let mut game = build_game(spec_json)?;
        game.state = serde_json::from_value(state.clone())
            .map_err(|_| "the saved state does not fit these rules".to_string())?;
        if game.state.current_turn as usize >= game.turn_order.len() {
            return Err("the saved state does not fit these rules".to_string());
        }
        if let GamePhase::Moving { position, .. } = &game.state.phase {
            // Recomputing re-enters the moving phase and would clear a jump lock, so the
            // saved phase is put back afterwards. The moves were computed under it.
            let phase = game.state.phase.clone();
            let position = position.clone();
            game.transition(GameTransition::CalculateMoves { position })
                .map_err(|e| format!("{e:?}"))?;
            game.state.phase = phase;
        }
        Ok(Session {
            game,
            spec_json: spec_json.to_string(),
            revision,
        })
    }

    /// The part of the game worth storing: everything except what can be recomputed.
    pub fn state(&self) -> Value {
        serde_json::to_value(&self.game.state).expect("a game state always serializes")
    }

    /// The revision this session was resumed at (0 for a new game).
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn reset(&mut self) {
        // The text was validated in `new`, so this cannot fail.
        if let Ok(game) = build_game(&self.spec_json) {
            self.game = game;
            self.revision = 0;
        }
    }

    fn ensure_playable(&self) -> Result<(), String> {
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
        if let GamePhase::Moving {
            position,
            locked: true,
            ..
        } = &self.game.state.phase
        {
            if position.as_slice() != [x, y] {
                return Err("finish the jump with the selected piece first".to_string());
            }
            // Picking the jumping piece again changes nothing. Asking the engine would
            // recompute its moves and clear the lock.
            return Ok(());
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

    /// Plays a piece from the hand onto the empty square `(x, y)`. The piece is picked and put
    /// down in one step, so a drop never leaves the game half way. Everything is checked
    /// first: on error the game is left exactly as it was.
    pub fn drop_piece(&mut self, code: &str, x: u8, y: u8) -> Result<(), String> {
        self.ensure_playable()?;
        if !self.can_drop() {
            return Err("finish what is pending before dropping a piece".to_string());
        }
        let player = self.game.current_player();
        let in_hand = self
            .game
            .hand()
            .get(&player)
            .and_then(|hand| hand.get(code))
            .copied()
            .unwrap_or(0);
        if in_hand == 0 {
            return Err("that piece is not in your hand".to_string());
        }
        let position = vec![x, y];
        if !self
            .game
            .compute_drop_squares(code, &player)
            .contains_key(&position)
        {
            return Err("not a legal square for that piece".to_string());
        }
        self.game
            .transition(GameTransition::CalculateDrops {
                piece_code: code.to_string(),
            })
            .map_err(|e| format!("{e:?}"))?;
        self.game
            .transition(GameTransition::ExecuteDrop { position })
            .map_err(|e| format!("{e:?}"))
    }

    /// A piece may be dropped unless a promotion is waiting for an answer or a jump is under
    /// way (a picked-up piece that is not mid-jump is simply put back).
    fn can_drop(&self) -> bool {
        !matches!(
            self.game.state.phase,
            GamePhase::Transforming { .. } | GamePhase::Moving { locked: true, .. }
        )
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
            GamePhase::Moving { position, .. } => {
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

        let mut hand: BTreeMap<String, BTreeMap<String, u32>> = BTreeMap::new();
        let mut drops: BTreeMap<String, Vec<Square>> = BTreeMap::new();
        if game.hand_enabled {
            for player in &game.players {
                let held: BTreeMap<String, u32> = game
                    .hand()
                    .get(player)
                    .map(|pieces| {
                        pieces
                            .iter()
                            .filter(|(_, count)| **count > 0)
                            .map(|(code, count)| (code.clone(), *count))
                            .collect()
                    })
                    .unwrap_or_default();
                hand.insert(player.clone(), held);
            }
            if status == Status::Playing && self.can_drop() {
                let turn = game.current_player();
                for code in hand.get(&turn).into_iter().flat_map(|held| held.keys()) {
                    let mut squares: Vec<Square> = game
                        .compute_drop_squares(code, &turn)
                        .keys()
                        .filter_map(|p| square(p))
                        .collect();
                    squares.sort();
                    drops.insert(code.clone(), squares);
                }
            }
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
            hand,
            drops,
        }
    }
}
