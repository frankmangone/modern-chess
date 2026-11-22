use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use modern_chess::logic::{Answer, ChoiceKind, Game, GamePhase, GameTransition, Way};
use modern_chess::shared::Position;
use modern_chess::specs::{parse_game_spec_str, BoardShape, HexLayout, PlaceKind};

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

/// The action a pass is, in a spec that allows passing.
const PASS: &str = "PASS";

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

    /// Takes back the answers given so far (a picked-up piece is put back). Refused, with
    /// `pending` as the reason, while what they do must be finished: a jump under way or a
    /// pending promotion.
    fn put_back(game: &mut Game, pending: &str) -> Result<(), String> {
        match &game.state.phase {
            GamePhase::Choosing { chosen } if !chosen.is_empty() => game
                .transition(GameTransition::Cancel)
                .map_err(|_| pending.to_string()),
            _ => Ok(()),
        }
    }

    /// Answers the next choice of the turn.
    fn choose(game: &mut Game, answer: Answer) -> Result<(), String> {
        game.transition(GameTransition::Choose(answer))
            .map_err(|e| format!("{e:?}"))
    }

    /// Selects the piece at `position`, putting back a piece picked up before.
    /// On error the game is left exactly as it was.
    pub fn select(&mut self, position: &str) -> Result<(), String> {
        let position = self.node_named(position)?;
        self.ensure_playable()?;
        // Mid-jump, only the jumping piece may be picked, and picking it again changes
        // nothing. The engine would refuse with a bare error, which means nothing to a reader.
        if let Some(jumping) = &self.game.state.continuing {
            if *jumping != position {
                return Err("finish the jump with the selected piece first".to_string());
            }
            return Ok(());
        }
        self.atomically(|game| {
            Self::put_back(game, "choose a promotion first")?;
            Self::choose(game, Answer::Place(position))
        })
    }

    /// Plays the selected piece to `position`. An answer that is not one of the selected
    /// piece's destinations changes nothing.
    pub fn make_move(&mut self, position: &str) -> Result<(), String> {
        let position = self.node_named(position)?;
        self.ensure_playable()?;
        let is_destination = self
            .game
            .choice()
            .places
            .iter()
            .any(|place| place.r#for == ChoiceKind::Move && place.name == position);
        if !is_destination {
            return Err("not a legal destination".to_string());
        }
        Self::choose(&mut self.game, Answer::Place(position))
    }

    /// Plays a piece from the hand, or from a supply, onto the empty square `position`. The
    /// piece is picked and put down in one step, so a drop never leaves the game half way.
    /// Everything is checked first: on error the game is left exactly as it was.
    pub fn drop_piece(&mut self, code: &str, position: &str) -> Result<(), String> {
        let position = self.node_named(position)?;
        self.ensure_playable()?;
        if !self.can_drop() {
            return Err("finish what is pending before dropping a piece".to_string());
        }
        let drops = self.drops();
        let Some(squares) = drops.get(code) else {
            return Err("that piece is not in your hand".to_string());
        };
        if !squares.contains(position.as_str()) {
            return Err("not a legal square for that piece".to_string());
        }
        let supplied = self.ways().iter().any(|way| way.action == code);
        let first = match supplied {
            true => Answer::Action(code.to_string()),
            false => Answer::Hand(code.to_string()),
        };
        self.atomically(|game| {
            Self::put_back(game, "finish what is pending before dropping a piece")?;
            Self::choose(game, first)?;
            Self::choose(game, Answer::Place(position))
        })
    }

    /// The turn's ways, whatever has been answered.
    fn ways(&self) -> &[Way] {
        self.game.state.ways.as_deref().unwrap_or_default()
    }

    /// Whether the answers so far can be taken back: no jump is under way, and nothing they
    /// do is showing yet (a pending promotion shows the moved piece).
    fn can_take_back(&self) -> bool {
        self.game.state.continuing.is_none()
            && (self.chosen().is_empty() || self.game.shown_pieces() == self.game.state.pieces)
    }

    /// A piece may be dropped unless a promotion is waiting for an answer or a jump is under
    /// way (a picked-up piece that is not mid-jump is simply put back).
    fn can_drop(&self) -> bool {
        self.can_take_back()
    }

    /// For the player to move: each piece code they hold, and each code of a supply, with the
    /// squares the turn's ways put it on, by name.
    fn drops(&self) -> BTreeMap<String, BTreeSet<Position>> {
        let game = &self.game;
        let turn = game.current_player();
        let mut drops: BTreeMap<String, BTreeSet<Position>> = BTreeMap::new();
        if game.rules.hand_enabled {
            let held = game.hand().get(&turn).into_iter().flatten();
            for (code, count) in held {
                if *count > 0 {
                    drops.insert(code.clone(), BTreeSet::new());
                }
            }
        }
        for way in self.ways() {
            let (code, square) = match (way.answers.as_slice(), way.kinds.as_slice()) {
                ([Answer::Hand(code), Answer::Place(square)], [ChoiceKind::Hand, _]) => {
                    (code, square)
                }
                ([Answer::Place(square)], [ChoiceKind::Place]) => (&way.action, square),
                _ => continue,
            };
            let held = drops.contains_key(code);
            let supplied = matches!(way.kinds.as_slice(), [ChoiceKind::Place]);
            if held || supplied {
                drops
                    .entry(code.clone())
                    .or_default()
                    .insert(square.clone());
            }
        }
        drops
    }

    /// The codes of a supply: each action that puts a new piece on a chosen place and asks
    /// nothing else, named after the code it puts.
    fn supply(&self) -> Vec<String> {
        let mut supply: Vec<String> = self
            .ways()
            .iter()
            .filter(|way| matches!(way.kinds.as_slice(), [ChoiceKind::Place]))
            .map(|way| way.action.clone())
            .collect();
        supply.sort();
        supply.dedup();
        supply
    }

    /// Gives up the turn. A piece that was picked up is put back; a promotion or a jump that is
    /// pending has to be finished first.
    pub fn pass(&mut self) -> Result<(), String> {
        self.ensure_playable()?;
        let allowed = self
            .ways()
            .iter()
            .any(|way| way.action == PASS && way.answers.is_empty());
        if !allowed {
            return Err("passing is not allowed in this game".to_string());
        }
        if !self.can_drop() {
            return Err("finish what is pending before passing".to_string());
        }
        self.atomically(|game| {
            Self::put_back(game, "finish what is pending before passing")?;
            Self::choose(game, Answer::Action(PASS.to_string()))
        })
    }

    /// Answers a pending promotion choice.
    pub fn promote(&mut self, code: &str) -> Result<(), String> {
        Self::choose(&mut self.game, Answer::Code(code.to_string()))
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
            .map(|map| map.lines.clone())
            .unwrap_or_default();

        let shown = game.shown_pieces();
        let mut pieces: Vec<(&Position, PieceView)> = shown
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

        let status = match &game.state.phase {
            GamePhase::Choosing { .. } => Status::Playing,
            GamePhase::GameOver { winner } => match winner {
                Some(winner) => Status::Won {
                    winner: winner.clone(),
                },
                None => Status::Draw,
            },
        };
        let choice = game.choice();
        let selected = choice
            .start
            .as_ref()
            .map(|start| start.as_str().to_string());
        let moves_to = choice
            .places
            .iter()
            .filter(|place| place.r#for == ChoiceKind::Move)
            .map(|place| &place.name);
        let destinations: Vec<String> = sorted_names(moves_to);
        let promotion = (!choice.codes.is_empty()).then(|| choice.codes.clone());

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

        let mut hand: BTreeMap<String, BTreeMap<String, u32>> = BTreeMap::new();
        let mut drops: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let supply = self.supply();
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
        // With the answers so far taken back, every way of the turn is open again.
        let open = status == Status::Playing && self.can_take_back();
        if open {
            for (code, squares) in self.drops() {
                drops.insert(code, sorted_names(squares.iter()));
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
        // A pass asks nothing, so it plays at once whenever its way is open.
        let can_pass = open
            && self
                .ways()
                .iter()
                .any(|way| way.action == PASS && way.answers.is_empty());
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
