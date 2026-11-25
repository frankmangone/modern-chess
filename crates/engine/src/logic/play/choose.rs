use crate::shared::IdMap;
use std::collections::BTreeMap;

use crate::logic::game::state::{board_changes, Asked, StageState};
use crate::logic::play::ways::answered;
use crate::logic::rules::steps::AskWho;
use crate::logic::game::state::move_record::RolledRecord;
use crate::logic::{Answer, ChoiceKind, Game, GameError, GamePhase, MoveRecord, Piece, Way};
use crate::shared::{BoardChange, Position, Symbol};
use crate::specs::Visible;

impl Game {
    /// Answers the next choice of the turn. The answer must lead to at least one of the turn's
    /// ways, or nothing changes. The way is played once the answers match exactly one
    /// complete way: when no longer way and no other action remains, or when the answer names
    /// the action of a complete way.
    pub fn choose(&mut self, answer: Answer) -> Result<(), GameError> {
        let GamePhase::Choosing { chosen } = &self.state.phase else {
            return Err(GameError::InvalidGamePhase);
        };
        let mut given = chosen.clone();
        given.push(answer);
        if self.state.ways.is_none() {
            self.work_out_ways();
        }

        let next = answered(&given);
        let mut any = false;
        let mut longer = false;
        let mut complete: Option<&Way> = None;
        let mut one_action = true;
        for way in self.ways_following(&given) {
            any = true;
            if way.answers.len() > next {
                longer = true;
                continue;
            }
            match complete {
                None => complete = Some(way),
                Some(first) => one_action &= first.action == way.action,
            }
        }
        if !any {
            return Err(GameError::NotAnOption);
        }
        let by_name = matches!(given.last(), Some(Answer::Action(_)));
        let play = complete
            .filter(|_| one_action && (!longer || by_name))
            .cloned();
        match play {
            Some(way) => self.play_way(way),
            None => self.state.phase = GamePhase::Choosing { chosen: given },
        }
        Ok(())
    }

    /// Plays `way` for the player to move. A way that runs `STAGE` keeps the turn with the same
    /// player, in that stage, when they have a way it allows (with the piece chosen, when every
    /// way starts with the same piece). A way that runs `ASK` puts the seats it asks to move, one
    /// after another, each in the stage it names; the seats with no way there are skipped, and
    /// once the last has answered the asker moves again in the `then` stage, if it has a way, or
    /// else the turn passes. A seat an `ASK` asked answers with one way, and that way may `STAGE`
    /// or `ASK` again. Otherwise the turn passes.
    fn play_way(&mut self, way: Way) {
        if self.state.focus.is_some() {
            self.play_together_way(way);
            return;
        }
        let player = self.current_player();
        self.apply_way(&way, &player);

        self.state.stage = None;

        // An asked seat has answered: it is done, unless its way keeps it in a stage.
        let answered = !self.state.asked.is_empty();
        if answered {
            self.state.asked.remove(0);
        }
        let mut last = None;
        if let Some(name) = way.stage {
            if answered {
                self.state.asked.insert(
                    0,
                    Asked {
                        player: player.clone(),
                        stage: name,
                    },
                );
            } else {
                // The player's own turn goes on, as before there was `ASK`.
                if self.enter_stage(name, way.last) {
                    return;
                }
                self.next_turn();
                self.check_game_over();
                return;
            }
            last = way.last;
        }
        if let Some(ask) = way.ask.as_ref().filter(|ask| ask.together) {
            let names = self.seats_asked(&ask.who, &player);
            if self.start_together(ask, names) {
                return;
            }
            // No seat has a way in the stage: nothing was asked, and the turn goes on.
            self.state.ask_then = ask.then;
        }
        if let Some(ask) = way.ask.as_ref().filter(|ask| !ask.together) {
            let mut seats: Vec<Asked> = self
                .seats_asked(&ask.who, &player)
                .into_iter()
                .map(|player| Asked {
                    player,
                    stage: ask.stage,
                })
                .collect();
            // A seat's answer that asks again is answered before the seats still waiting.
            seats.append(&mut self.state.asked);
            self.state.asked = seats;
            if !answered {
                self.state.ask_then = ask.then;
            }
        }
        if self.advance_asks(last) {
            return;
        }
        self.next_turn();
        self.check_game_over();
    }

    /// Makes the changes of `way`, played by `player`, and records it in the history.
    pub(super) fn apply_way(&mut self, way: &Way, player: &str) {
        let record = self.record_of(way, player);
        let before = self.piece_counts();
        board_changes::apply_to_map(&mut self.state.pieces, &way.changes);
        self.record_losses(&before);
        self.apply_holder_changes(way);
        self.apply_counts(way, player);
        self.state.draws = self.state.draws.saturating_add(u64::from(way.draws));
        self.state.history.push(record);
        self.state.ways = None;
    }

    /// Puts the stage `name` on the player to move, and works out their ways in it. True when they
    /// have one, with the choice begun (the piece chosen already, when every way starts with the
    /// same piece); otherwise the stage is left again, and nothing else changes.
    pub(super) fn enter_stage(&mut self, name: Symbol, last: Option<Position>) -> bool {
        self.state.stage = Some(StageState { name, last });
        let ways = self.compute_ways();
        if ways.is_empty() {
            self.state.stage = None;
            return false;
        }
        let chosen = stage_pin(&ways)
            .map(|piece| vec![Answer::Place(piece)])
            .unwrap_or_default();
        self.state.ways = Some(ways);
        self.state.phase = GamePhase::Choosing { chosen };
        true
    }

    /// Moves on to the next seat an `ASK` is waiting on that has a way in its stage, dropping the
    /// seats that have none. When none is left the asker moves again in the `then` stage. True when
    /// someone is to answer; false when the turn is to be handed on. `last` is where the way before
    /// ended, for the first seat only.
    pub(super) fn advance_asks(&mut self, mut last: Option<Position>) -> bool {
        while let Some(asked) = self.state.asked.first() {
            let stage = asked.stage;
            if self.enter_stage(stage, last.take()) {
                return true;
            }
            self.state.asked.remove(0);
        }
        match self.state.ask_then.take() {
            Some(then) => self.enter_stage(then, None),
            None => false,
        }
    }

    /// The seats `who` names, for the player `asker` to ask, in the order they answer. A player
    /// who has left the game is not asked, and no seat twice.
    pub(super) fn seats_asked(&self, who: &AskWho, asker: &str) -> Vec<String> {
        let order = &self.state.turn_order;
        let mut seats: Vec<String> = match who {
            AskWho::Mover => vec![asker.to_string()],
            AskWho::Named(names) => names.clone(),
            AskWho::Others | AskWho::All => {
                let at = if asker == self.asker_ref() {
                    usize::from(self.state.current_turn)
                } else {
                    order.iter().position(|name| name == asker).unwrap_or(0)
                };
                let mut seats: Vec<String> = order
                    .iter()
                    .cycle()
                    .skip(at + 1)
                    .take(order.len())
                    .filter(|name| name.as_str() != asker)
                    .cloned()
                    .collect();
                if matches!(who, AskWho::All) {
                    seats.push(asker.to_string());
                }
                seats
            }
        };
        let mut seen: Vec<String> = Vec::new();
        seats.retain(|name| {
            let fresh = order.contains(name) && !seen.contains(name);
            if fresh {
                seen.push(name.clone());
            }
            fresh
        });
        seats
    }

    /// Makes the counter changes of `way`, played by `player`.
    fn apply_counts(&mut self, way: &Way, player: &str) {
        for change in &way.counts {
            let players: Vec<String> = if change.all {
                self.rules.players.clone()
            } else {
                vec![player.to_string()]
            };
            for name in players {
                let value = self
                    .state
                    .counters
                    .entry(name)
                    .or_default()
                    .entry(change.counter.clone())
                    .or_insert(0);
                *value = if change.reset {
                    0
                } else {
                    value.saturating_add(change.by)
                };
            }
        }
    }

    /// The history record of `way`, played by `player` from the board as it stands.
    fn record_of(&self, way: &Way, player: &str) -> MoveRecord {
        let answered_as = |wanted: &[ChoiceKind]| {
            way.kinds
                .iter()
                .zip(&way.answers)
                .find_map(|(kind, answer)| match answer {
                    Answer::Place(place) if wanted.contains(kind) => Some(*place),
                    _ => None,
                })
        };
        let from = answered_as(&[ChoiceKind::Piece]);
        let to = answered_as(&[ChoiceKind::Move, ChoiceKind::Place]);
        let held = way.answers.iter().find_map(|answer| match answer {
            Answer::Held(code) => Some(code.clone()),
            _ => None,
        });
        let put_on_to = || {
            let to = to.as_ref()?;
            let change = way.changes.iter().rev().find(|c| c.position == *to)?;
            change.piece.as_ref().map(|piece| piece.code.to_string())
        };
        let piece_code = from
            .as_ref()
            .and_then(|from| self.state.pieces.get(from))
            .map(|piece| piece.code.to_string())
            .or_else(|| way.from_hand.map(String::from))
            .or(held)
            .or_else(put_on_to)
            .unwrap_or_default();
        MoveRecord {
            player: player.to_string(),
            action: way.action.clone(),
            chosen: way.answers.clone(),
            piece_code,
            from: from.unwrap_or_default(),
            to: to.unwrap_or_default(),
            captures: way.captures,
            passed: way.changes.is_empty() && way.holder_changes.is_empty(),
            private_bags: self.private_bags_of(way),
            rolled: way
                .rolled
                .iter()
                .map(|die| RolledRecord {
                    bag: die.at.to_string(),
                    face: die
                        .face
                        .iter()
                        .map(|(code, count)| (code.to_string(), *count))
                        .collect(),
                })
                .collect(),
            shuffled: way.shuffled.iter().map(|pile| pile.to_string()).collect(),
        }
    }

    /// The bags `way` reads (a `HELD` choice), changes or rolls into that not everyone sees into,
    /// by name and in the order met. A game that hides nothing has none.
    fn private_bags_of(&self, way: &Way) -> Vec<String> {
        let mut bags: Vec<String> = Vec::new();
        let touched = way
            .held_bags
            .iter()
            .chain(way.holder_changes.iter().map(|change| &change.at))
            .chain(way.rolled.iter().map(|die| &die.at));
        for at in touched {
            let hidden = self
                .rules
                .holders
                .get(at)
                .is_some_and(|info| info.visible != Visible::All);
            let name = at.to_string();
            if hidden && !bags.contains(&name) {
                bags.push(name);
            }
        }
        bags
    }

    /// Adds to and takes out of the holders what `way` does.
    fn apply_holder_changes(&mut self, way: &Way) {
        for change in &way.holder_changes {
            let holder = self.state.holders.entry(change.at).or_default();
            change.apply_to(holder);
            if holder.is_empty() {
                self.state.holders.remove(&change.at);
            }
        }
    }

    /// How many pieces of each kind every player has on the board, as (player, code) → count.
    fn piece_counts(&self) -> BTreeMap<(Symbol, Symbol), u32> {
        let mut counts = BTreeMap::new();
        for piece in self.state.pieces.values() {
            *counts.entry((piece.player, piece.code)).or_insert(0) += 1;
        }
        counts
    }

    /// Adds to `captured` whatever a move took off the board, given the counts from before it.
    /// A piece that only changed form (a promotion) or place (a castling rook) leaves the same
    /// number of pieces behind, so each piece a player gained cancels one it lost.
    fn record_losses(&mut self, before: &BTreeMap<(Symbol, Symbol), u32>) {
        let after = self.piece_counts();
        let mut gains: BTreeMap<Symbol, u32> = BTreeMap::new();
        for ((player, code), now) in &after {
            let was = before.get(&(*player, *code)).copied().unwrap_or(0);
            if *now > was {
                *gains.entry(*player).or_insert(0) += now - was;
            }
        }
        for ((player, code), was) in before {
            let now = after.get(&(*player, *code)).copied().unwrap_or(0);
            if now >= *was {
                continue;
            }
            let mut lost = was - now;
            if let Some(gain) = gains.get_mut(player) {
                let cancelled = lost.min(*gain);
                lost -= cancelled;
                *gain -= cancelled;
            }
            if lost > 0 {
                *self
                    .state
                    .captured
                    .entry(player.to_string())
                    .or_default()
                    .entry(code.to_string())
                    .or_insert(0) += lost;
            }
        }
    }

    /// The changes the answers so far have made: those every remaining way makes before its
    /// next answer, when they are the same for all of them. Empty when nothing is answered, or
    /// when the ways disagree.
    pub(crate) fn shown_changes(&self) -> &[BoardChange] {
        let chosen = self.chosen();
        let next = answered(chosen);
        if next == 0 {
            return &[];
        }
        let mut agreed: Option<&[BoardChange]> = None;
        for way in self.ways_following(chosen) {
            let made = way
                .changes
                .get(..way.changes_before(next))
                .unwrap_or_default();
            match agreed {
                None => agreed = Some(made),
                Some(earlier) if same_changes(earlier, made) => {}
                Some(_) => return &[],
            }
        }
        agreed.unwrap_or_default()
    }

    /// The board with the effects of the answers so far applied, while every remaining way
    /// agrees on them; otherwise the board itself.
    pub fn shown_pieces(&self) -> IdMap<Position, Piece> {
        board_changes::simulate_from(&self.state.pieces, self.shown_changes())
    }
}

fn same_changes(a: &[BoardChange], b: &[BoardChange]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(a, b)| a.position == b.position && a.piece == b.piece)
}

/// The piece every one of `ways` starts with, when they all start with the same: a stage whose
/// ways are one piece's (a jump) has that piece chosen already, and cannot be cancelled.
pub(super) fn stage_pin(ways: &[Way]) -> Option<Position> {
    let first = ways.first()?;
    let piece = match (first.kinds.first(), first.answers.first()) {
        (Some(ChoiceKind::Piece), Some(Answer::Place(piece))) => *piece,
        _ => return None,
    };
    ways.iter()
        .all(|way| {
            way.kinds.first() == Some(&ChoiceKind::Piece)
                && way.answers.first() == Some(&Answer::Place(piece))
        })
        .then_some(piece)
}
