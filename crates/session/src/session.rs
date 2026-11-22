use std::collections::BTreeMap;

use serde_json::Value;

use modern_chess::logic::{CellConditionKind, Game, GamePhase, GameTransition};
use modern_chess::shared::Position;
use modern_chess::specs::{parse_game_spec_str, BoardShape, HexLayout};

use crate::view::{Layout, NodeView, PieceView, Shape, Status, View};

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

const NOT_ON_THE_BOARD: &str = "that square is not on the board";

/// The names of `positions`, in board order.
fn sorted_names<'a>(positions: impl Iterator<Item = &'a Position>) -> Vec<String> {
    let mut positions: Vec<&Position> = positions.collect();
    positions.sort();
    positions
        .into_iter()
        .map(|p| p.as_str().to_string())
        .collect()
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
        game.restore(
            serde_json::from_value(state.clone())
                .map_err(|_| "the saved state does not fit these rules".to_string())?,
        );
        if game.state.current_turn as usize >= game.state.turn_order.len() {
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

    /// The position `name` spells, when it is a node of the board.
    fn node_named(&self, name: &str) -> Result<Position, String> {
        let position = Position::from(name);
        match self.game.board.node(&position) {
            Some(_) => Ok(position),
            None => Err(NOT_ON_THE_BOARD.to_string()),
        }
    }

    fn ensure_playable(&self) -> Result<(), String> {
        if matches!(self.game.state.phase, GamePhase::GameOver { .. }) {
            return Err("the game is over".to_string());
        }
        Ok(())
    }

    /// Selects the piece at `position` and computes its legal destinations.
    /// On error the game is left exactly as it was.
    pub fn select(&mut self, position: &str) -> Result<(), String> {
        let position = self.node_named(position)?;
        self.ensure_playable()?;
        if matches!(self.game.state.phase, GamePhase::Transforming { .. }) {
            return Err("choose a promotion first".to_string());
        }
        // Mid-jump, only the jumping piece may be picked. The engine would refuse with
        // a bare `InvalidPlayer`, which means nothing to a reader.
        if let GamePhase::Moving {
            position: picked,
            locked: true,
            ..
        } = &self.game.state.phase
        {
            if *picked != position {
                return Err("finish the jump with the selected piece first".to_string());
            }
            // Picking the jumping piece again changes nothing. Asking the engine would
            // recompute its moves and clear the lock.
            return Ok(());
        }
        self.game
            .transition(GameTransition::CalculateMoves { position })
            .map_err(|e| format!("{e:?}"))
    }

    /// Plays the selected piece to `position`. The engine advances the turn.
    /// The destination is checked here first: the engine resets to idle on an
    /// invalid destination, which would drop a jump in progress.
    pub fn make_move(&mut self, position: &str) -> Result<(), String> {
        let position = self.node_named(position)?;
        self.ensure_playable()?;
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

    /// Plays a piece from the hand onto the empty square `position`. The piece is picked and put
    /// down in one step, so a drop never leaves the game half way. Everything is checked
    /// first: on error the game is left exactly as it was.
    pub fn drop_piece(&mut self, code: &str, position: &str) -> Result<(), String> {
        let position = self.node_named(position)?;
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
        if in_hand == 0 && !self.game.rules.unlimited_supply.contains(code) {
            return Err("that piece is not in your hand".to_string());
        }
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

    /// Gives up the turn. A piece that was picked up is put back; a promotion or a jump that is
    /// pending has to be finished first.
    pub fn pass(&mut self) -> Result<(), String> {
        self.ensure_playable()?;
        if !self.game.rules.allow_pass {
            return Err("passing is not allowed in this game".to_string());
        }
        if !self.can_drop() {
            return Err("finish what is pending before passing".to_string());
        }
        self.game
            .transition(GameTransition::Pass)
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

        let nodes: Vec<NodeView> = game
            .board
            .all_positions()
            .into_iter()
            .filter_map(|position| match game.board.coordinates(&position)? {
                [x, y] => Some(NodeView {
                    name: position.as_str().to_string(),
                    at: [*x, *y],
                }),
                _ => None,
            })
            .collect();

        let mut pieces: Vec<(&Position, PieceView)> = game
            .state
            .pieces
            .iter()
            .map(|(position, piece)| {
                (
                    position,
                    PieceView {
                        position: position.as_str().to_string(),
                        code: piece.code.clone(),
                        player: piece.player.clone(),
                    },
                )
            })
            .collect();
        pieces.sort_by(|a, b| a.0.cmp(b.0));
        let pieces: Vec<PieceView> = pieces.into_iter().map(|(_, piece)| piece).collect();

        let mut selected = None;
        let mut promotion = None;
        let mut status = Status::Playing;
        match &game.state.phase {
            GamePhase::Moving { position, .. } => {
                selected = Some(position.as_str().to_string());
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

        let destinations: Vec<String> = match (&selected, &game.state.available_moves) {
            (Some(_), Some(moves)) => sorted_names(moves.keys()),
            _ => Vec::new(),
        };

        let mut zones: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
        for (name, condition) in &game.rules.conditions {
            if condition.kind != CellConditionKind::Position {
                continue;
            }
            let per_player = condition
                .check
                .iter()
                .map(|(player, names)| {
                    let on_board: Vec<Position> = names
                        .iter()
                        .map(|n| Position::from(n.as_str()))
                        .filter(|p| game.board.node(p).is_some())
                        .collect();
                    (player.clone(), sorted_names(on_board.iter()))
                })
                .collect();
            zones.insert(name.clone(), per_player);
        }

        let mut hand: BTreeMap<String, BTreeMap<String, u32>> = BTreeMap::new();
        let mut drops: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut supply: Vec<String> = game.rules.unlimited_supply.iter().cloned().collect();
        supply.sort();
        if game.rules.hand_enabled {
            for player in &game.rules.players {
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
        }
        if status == Status::Playing && self.can_drop() {
            let turn = game.current_player();
            let held = hand.get(&turn).into_iter().flat_map(|held| held.keys());
            for code in held.chain(supply.iter()) {
                let squares = sorted_names(game.compute_drop_squares(code, &turn).keys());
                drops.insert(code.clone(), squares);
            }
        }

        let captured: BTreeMap<String, BTreeMap<String, u32>> = game
            .rules
            .players
            .iter()
            .map(|player| {
                let lost = game
                    .state
                    .captured
                    .get(player)
                    .map(|pieces| pieces.iter().map(|(code, n)| (code.clone(), *n)).collect())
                    .unwrap_or_default();
                (player.clone(), lost)
            })
            .collect();

        let mut material: BTreeMap<String, u32> = BTreeMap::new();
        if !game.rules.piece_values.is_empty() {
            for player in &game.rules.players {
                material.insert(player.clone(), 0);
            }
            let worth = |code: &str| game.rules.piece_values.get(code).copied().unwrap_or(0);
            for piece in game.state.pieces.values() {
                let total = material.entry(piece.player.clone()).or_insert(0);
                *total = total.saturating_add(worth(&piece.code));
            }
            for (player, held) in &hand {
                for (code, count) in held {
                    let total = material.entry(player.clone()).or_insert(0);
                    *total = total.saturating_add(worth(code).saturating_mul(*count));
                }
            }
        }

        let score: BTreeMap<String, f64> = game.area_scores().unwrap_or_default();
        let can_pass = game.rules.allow_pass && status == Status::Playing && self.can_drop();
        let passes = game.consecutive_passes() as u32;

        View {
            nodes,
            shape: match game.board.shape {
                BoardShape::Square => Shape::Square,
                BoardShape::Hex { .. } => Shape::Hex,
            },
            layout: match game.board.shape {
                BoardShape::Square => None,
                BoardShape::Hex {
                    layout: HexLayout::Rhombus,
                    ..
                } => Some(Layout::Rhombus),
                BoardShape::Hex {
                    layout: HexLayout::Hexagon,
                    ..
                } => Some(Layout::Hexagon),
            },
            players: game.rules.players.clone(),
            pieces,
            turn: game.current_player(),
            selected,
            destinations,
            promotion,
            status,
            zones,
            hands: game.rules.hand_enabled,
            hand,
            drops,
            captured,
            material,
            supply,
            can_pass,
            passes,
            score,
        }
    }
}
