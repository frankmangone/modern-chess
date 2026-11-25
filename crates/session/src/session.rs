use std::collections::BTreeMap;

use serde_json::Value;

use modern_chess::logic::places::holders::{bag_position, HolderOwner};
use modern_chess::logic::{Answer, Choice, Game, GamePhase, GameTransition, MoveRecord};
use modern_chess::shared::{chance, Position};
use modern_chess::specs::{parse_game_spec_str, BoardShape, HexLayout, PlaceKind, Visible};

use crate::view::{BagView, Layout, NodeView, PieceView, Shape, Status, View};
use crate::viewer::Viewer;

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
    /// A new game with no seed: a game that rolls or shuffles plays the sequence of seed 0. Fine
    /// for tests and tools; whoever hosts a real game uses `new_with_seed`.
    pub fn new(spec_json: &str) -> Result<Session, String> {
        Ok(Session {
            game: build_game(spec_json)?,
            spec_json: spec_json.to_string(),
            revision: 0,
        })
    }

    /// A new game whose chance (dice, shuffles) follows `seed`. A game that never rolls or
    /// shuffles ignores it. The seed is part of the saved `state()`, never of the `view()`.
    pub fn new_with_seed(spec_json: &str, seed: u64) -> Result<Session, String> {
        let mut session = Session::new(spec_json)?;
        session.game.set_seed(seed);
        Ok(session)
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
        if game.state.current_turn as usize >= game.state.turn_order.len()
            || !game.stage_fits_the_rules()
        {
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

    /// Starts the game over. A game with a seed is dealt another one, worked out from the old
    /// (so a restart does not replay the same dice).
    pub fn reset(&mut self) {
        // The text was validated in `new`, so this cannot fail.
        if let Ok(mut game) = build_game(&self.spec_json) {
            if let Some(seed) = self.game.state.seed {
                game.set_seed(chance::next_seed(seed));
            }
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
        let jumping = self.game.pinned_piece();
        if let (Some(jumping), Some(place)) = (&jumping, &place) {
            if jumping == place && self.chosen() == [Answer::Place(*place)] {
                return Ok(());
            }
        }
        let jumping = jumping.is_some();
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

    /// What has been played so far, as `viewer` may know it. A die rolled into a bag they may not
    /// see into keeps its place in the list but loses its face, and a card taken from, or a code
    /// chosen for, such a bag by a choice loses its name: the die was rolled and the card was played,
    /// not which. A
    /// shuffle is only ever a name. When the game is over nothing is hidden.
    pub fn history_for(&self, viewer: &Viewer) -> Vec<MoveRecord> {
        let sees = |bag: &str| self.sees_bag(viewer, &Position::from(bag));
        self.game
            .history()
            .iter()
            .cloned()
            .map(|mut record| {
                for die in &mut record.rolled {
                    if !sees(&die.bag) {
                        die.face.clear();
                    }
                }
                if record.private_bags.iter().any(|bag| !sees(bag)) {
                    // The turn touched a bag the viewer may not see into: a card it took or
                    // a code it chose may be what went in or out of it.
                    for answer in &mut record.chosen {
                        if let Answer::Held(code) | Answer::Code(code) = answer {
                            code.clear();
                        }
                    }
                    // Without a piece picked from the board, the code is the held one.
                    if record.from == Position::default() {
                        record.piece_code.clear();
                    }
                }
                record
            })
            .collect()
    }

    /// Whether the game is over, when every bag is shown to everyone.
    fn is_over(&self) -> bool {
        matches!(self.game.state.phase, GamePhase::GameOver { .. })
    }

    /// Whether `viewer` may see what the bag at `at` keeps: it is a bag they see into, or the
    /// game is over. A place that is no bag is public.
    fn sees_bag(&self, viewer: &Viewer, at: &Position) -> bool {
        if self.is_over() {
            return true;
        }
        match self.game.rules.holders.get(at) {
            Some(info) => viewer.sees(
                info.visible,
                match info.owner {
                    HolderOwner::Player(player) => Some(player.as_str()),
                    HolderOwner::Shared | HolderOwner::Place => None,
                },
            ),
            None => true,
        }
    }

    /// The whole game as everyone at one table sees it: the hot seat of the "Try it" board, the
    /// book and the tests. The same as `view_for(&Viewer::Everyone)`; a game server never
    /// sends it to a player.
    pub fn view(&self) -> View {
        self.view_for(&Viewer::Everyone)
    }

    /// The game as `viewer` sees it. A bag they may not see into comes without its pieces (and
    /// with its count, if the bag shows one); the numbers that read such a bag are left out;
    /// and only the player to move has a choice. When the game is over nothing is hidden.
    pub fn view_for(&self, viewer: &Viewer) -> View {
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

        // The board as the mover's answers so far would leave it is the mover's to see. A game
        // that hides something does not show it to anyone else: it could be the hidden card.
        let hides_something = game
            .rules
            .holders
            .iter()
            .any(|(_, info)| info.visible != Visible::All);
        let shown = if hides_something && !viewer.is_mover(&game.current_player()) {
            game.state.pieces.clone()
        } else {
            game.shown_pieces()
        };
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

        let contents = game.bags();
        let seen_into = |bag: &modern_chess::logic::BagContents| {
            let at = bag_position(&bag.name, bag.owner.as_deref());
            self.sees_bag(viewer, &at)
        };
        let bags: Vec<BagView> = contents
            .iter()
            .map(|bag| {
                let pieces: BTreeMap<String, u32> = bag
                    .pieces
                    .iter()
                    .filter(|(_, n)| **n > 0)
                    .map(|(code, n)| (code.clone(), *n))
                    .collect();
                if seen_into(bag) {
                    return BagView {
                        name: bag.name.clone(),
                        owner: bag.owner.clone(),
                        pieces,
                        hidden: false,
                        count: None,
                    };
                }
                BagView {
                    name: bag.name.clone(),
                    owner: bag.owner.clone(),
                    pieces: BTreeMap::new(),
                    hidden: true,
                    count: bag.shows_count.then(|| pieces.values().sum()),
                }
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
            for bag in &contents {
                let Some(player) = &bag.owner else { continue };
                for (code, count) in &bag.pieces {
                    let total = material.entry(player.clone()).or_insert(0);
                    *total = total.saturating_add(worth(code).saturating_mul(*count));
                }
            }
            // A number that reads a bag the viewer may not see into is not theirs to know: a
            // player's total is withheld while any bag of theirs is hidden from the viewer.
            for (bag, shown) in contents.iter().zip(&bags) {
                if let (true, Some(player)) = (shown.hidden, &bag.owner) {
                    material.remove(player);
                }
            }
        }

        let mut score: BTreeMap<String, f64> = game.scores().unwrap_or_default();
        // Like material: a player's score is withheld while a bag it reads, of theirs, is hidden
        // from the viewer. A seat still sees its own total of its own hand. The whole number goes,
        // not only the term that reads the bag. A bag is matched by name and owner because a
        // `BAGGED` term may only name a bag each player has (the spec is refused otherwise).
        let read_by_score = game.score_bags();
        for shown in &bags {
            if let (true, Some(player), true) = (
                shown.hidden,
                &shown.owner,
                read_by_score.contains(&shown.name),
            ) {
                score.remove(player);
            }
        }
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
            choice: if viewer.is_mover(&game.current_player()) {
                game.choice()
            } else {
                Choice::default()
            },
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
