use std::collections::BTreeMap;

use serde_json::Value;

use modern_chess::logic::{Answer, Game, GamePhase, GameTransition};
use modern_chess::shared::Position;
use modern_chess::specs::{parse_game_spec_str, BoardShape, HexLayout, PlaceKind};

use crate::view::{BagView, Layout, NodeView, PieceView, Shape, Status, View};

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

const NOT_A_CHOICE: &str = "that is not a choice right now";

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
    /// on every request. The answers given so far (a piece that was picked up, a jump under
    /// way) are kept: restoring works the turn's ways out again, since the saved state does
    /// not carry them.
    pub fn resume(spec_json: &str, state: &Value, revision: u64) -> Result<Session, String> {
        let mut game = build_game(spec_json)?;
        game.restore(
            serde_json::from_value(state.clone())
                .map_err(|_| "the saved state does not fit these rules".to_string())?,
        );
        if game.state.current_turn as usize >= game.state.turn_order.len() {
            return Err("the saved state does not fit these rules".to_string());
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

    /// The answers given so far this turn.
    fn chosen(&self) -> &[Answer] {
        match &self.game.state.phase {
            GamePhase::Choosing { chosen } => chosen,
            GamePhase::GameOver { .. } => &[],
        }
    }

    /// Runs `step` on the game; when it fails, the game is put back exactly as it was.
    fn atomically(
        &mut self,
        step: impl FnOnce(&mut Game) -> Result<(), String>,
    ) -> Result<(), String> {
        let saved = self.game.state.clone();
        let result = step(&mut self.game);
        if result.is_err() {
            self.game.state = saved;
        }
        result
    }

    /// Answers the next choice of the turn, keeping today's click feel. An answer that fits
    /// the open choice is simply chosen. Otherwise the answers so far are taken back, when
    /// they can be, and the answer is tried on the fresh choice: a click on another piece
    /// picks that piece, and a click on a place that answers nothing only deselects. An answer
    /// that is no choice at all changes nothing.
    pub fn choose(&mut self, answer: Answer) -> Result<(), String> {
        let place = match &answer {
            Answer::Place(position) => Some(self.node_named(position.as_str())?),
            _ => None,
        };
        self.ensure_playable()?;
        // Mid-jump, picking the jumping piece again changes nothing. Anything else the
        // engine would refuse with a bare error, which means nothing to a reader.
        if let (Some(jumping), Some(place)) = (&self.game.state.continuing, &place) {
            if jumping == place && self.chosen() == [Answer::Place(*place)] {
                return Ok(());
            }
        }
        let jumping = self.game.state.continuing.is_some();
        self.atomically(|game| {
            if Self::choose_in(game, answer.clone()).is_ok() {
                return Ok(());
            }
            let cancelled = game.state.phase != GamePhase::Choosing { chosen: Vec::new() }
                && game.transition(GameTransition::Cancel).is_ok();
            if !cancelled {
                return Err(match jumping {
                    true => "finish the jump with the selected piece first".to_string(),
                    false => NOT_A_CHOICE.to_string(),
                });
            }
            match Self::choose_in(game, answer) {
                Ok(()) => Ok(()),
                Err(_) if place.is_some() => Ok(()),
                Err(_) => Err(NOT_A_CHOICE.to_string()),
            }
        })
    }

    fn choose_in(game: &mut Game, answer: Answer) -> Result<(), String> {
        game.transition(GameTransition::Choose(answer))
            .map_err(|e| format!("{e:?}"))
    }

    /// Takes back the answers given so far (a picked-up piece is put back). Refused while a
    /// jump is under way or once what the answers do is showing (a pending promotion).
    pub fn cancel(&mut self) -> Result<(), String> {
        self.ensure_playable()?;
        self.atomically(|game| {
            game.transition(GameTransition::Cancel)
                .map_err(|_| "that cannot be taken back now".to_string())
        })
    }

    /// The board's nodes in board order: a map's places with their kinds, or the grid's cells.
    fn nodes(&self) -> Vec<NodeView> {
        let board = &self.game.board;
        if let Some(map) = board.places() {
            return map
                .places
                .iter()
                .map(|place| NodeView {
                    name: place.name.clone(),
                    at: place.at,
                    kind: Some(place.kind),
                    ends: place.ends.clone(),
                    around: (place.kind == PlaceKind::Area).then(|| place.around.clone()),
                })
                .collect();
        }
        board
            .all_positions()
            .into_iter()
            .filter_map(|position| match board.coordinates(&position)? {
                [x, y] => Some(NodeView {
                    name: position.as_str().to_string(),
                    at: [f64::from(*x), f64::from(*y)],
                    kind: None,
                    ends: None,
                    around: None,
                }),
                _ => None,
            })
            .collect()
    }

    pub fn view(&self) -> View {
        let game = &self.game;

        let nodes = self.nodes();
        let lines = game
            .board
            .places()
            .map(|map| {
                map.lines
                    .iter()
                    .chain(map.one_way.iter())
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let shown = game.shown_pieces();
        let mut pieces: Vec<(&Position, PieceView)> = shown
            .iter()
            .map(|(position, piece)| {
                (
                    position,
                    PieceView {
                        position: position.as_str().to_string(),
                        code: piece.code.to_string(),
                        player: piece.player.to_string(),
                        count: None,
                    },
                )
            })
            .collect();
        pieces.sort_by(|a, b| a.0.cmp(b.0));
        let mut pieces: Vec<PieceView> = pieces.into_iter().map(|(_, piece)| piece).collect();
        // What the places that hold many keep: one entry for each owner and kind, with a count.
        let mut kept: Vec<PieceView> = game
            .state
            .holders
            .iter()
            .filter(|(at, _)| game.board.node(at).is_some())
            .flat_map(|(at, holder)| {
                holder.entries().iter().map(move |held| PieceView {
                    position: at.as_str().to_string(),
                    code: held.code.to_string(),
                    player: held.owner.to_string(),
                    count: Some(held.count),
                })
            })
            .collect();
        kept.sort_by(|a, b| {
            (&a.position, &a.code, &a.player).cmp(&(&b.position, &b.code, &b.player))
        });
        pieces.extend(kept);

        let status = match &game.state.phase {
            GamePhase::Choosing { .. } => Status::Playing,
            GamePhase::GameOver { winner } => match winner {
                Some(winner) => Status::Won {
                    winner: winner.clone(),
                },
                None => Status::Draw,
            },
        };
        let mut zones: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
        for (name, zone) in &game.rules.zones {
            let per_player = zone
                .places
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

        let bags: Vec<BagView> = game
            .bags()
            .into_iter()
            .map(|bag| BagView {
                name: bag.name,
                owner: bag.owner,
                pieces: bag.pieces.into_iter().filter(|(_, n)| *n > 0).collect(),
            })
            .collect();
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
                let total = material.entry(piece.player.to_string()).or_insert(0);
                *total = total.saturating_add(worth(&piece.code));
            }
            for bag in &bags {
                let Some(player) = &bag.owner else { continue };
                for (code, count) in &bag.pieces {
                    let total = material.entry(player.clone()).or_insert(0);
                    *total = total.saturating_add(worth(code).saturating_mul(*count));
                }
            }
        }

        let score: BTreeMap<String, f64> = game.scores().unwrap_or_default();
        let passes = game.consecutive_passes() as u32;

        View {
            nodes,
            lines,
            shape: match game.board.shape {
                BoardShape::Square => Shape::Square,
                BoardShape::Hex { .. } => Shape::Hex,
                BoardShape::Map => Shape::Map,
            },
            layout: match game.board.shape {
                BoardShape::Square | BoardShape::Map => None,
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
            choice: game.choice(),
            status,
            zones,
            bags,
            captured,
            material,
            passes,
            score,
        }
    }
}
