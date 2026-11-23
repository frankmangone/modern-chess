//! Helpers that drive a turn through the protocol (`Choose` and `Cancel`) the way the tests
//! drove it through picking a piece and playing a move, so each test still reads as it did.
//!
//! Tests set positions up by hand after the game was built, so the ways the engine worked out
//! when the turn started can be out of date. Before the first answer of a turn, the helpers
//! work the ways out again from the board as it stands, as picking a piece used to.
#![allow(dead_code)]

use std::sync::Arc;

use serde_json::{json, Value};

use crate::logic::blueprint::test_block::{judge, Test, TestContext};
use crate::logic::{Answer, ChoiceKind, Game, GameError, GamePhase, GameTransition, Labels, Way};
use crate::shared::Position;
use crate::specs::game::test_block::TestSpec;
use crate::specs::PlayerSpec;

/// The steps of a move onto an empty square only: the piece goes there.
pub fn onto_an_empty_square() -> Value {
    json!([
        { "test": "NOT", "of": {
            "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }
        } },
        { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
    ])
}

/// The steps of a move onto a square no ally holds: the piece goes there, taking what is there.
pub fn onto_a_square_without_an_ally() -> Value {
    json!([
        { "test": "NOT", "of": {
            "test": "COUNT",
            "in": { "region": "AT", "at": { "place": "TARGET" } },
            "where": { "owner": "ALLY" }
        } },
        { "effect": "MOVE", "from": { "place": "START" }, "to": { "place": "TARGET" } }
    ])
}

/// The steps of a move that captures an enemy by exploding: the enemy on `TARGET`, the piece
/// and every piece around `TARGET` are removed.
pub fn capturing_explosion() -> Value {
    json!([
        { "test": "COUNT",
          "in": { "region": "AT", "at": { "place": "TARGET" } },
          "where": { "owner": "ENEMY" } },
        { "effect": "REMOVE", "at": { "place": "TARGET" } },
        { "effect": "REMOVE", "at": { "place": "START" } },
        { "effect": "FOR_EACH",
          "in": { "region": "AROUND", "of": { "place": "TARGET" } },
          "do": [{
              "if": { "test": "COUNT", "in": { "region": "AT", "at": { "place": "EACH" } } },
              "then": [{ "effect": "REMOVE", "at": { "place": "EACH" } }]
          }] }
    ])
}

/// The action MOVE: pick a piece of the mover's, then one of its moves.
pub fn move_action() -> Value {
    json!({ "name": "MOVE", "do": [
        { "choose": "PIECE", "where": { "owner": "ALLY" } },
        { "choose": "MOVE" }
    ] })
}

/// The action named after `code`: put a new piece of that code on an empty square (a supply,
/// as the stones of Go).
pub fn place_action(code: &str) -> Value {
    json!({ "name": code, "do": [
        { "choose": "PLACE", "in": { "region": "BOARD" } },
        { "test": "NOT", "of": {
            "test": "COUNT", "in": { "region": "AT", "at": { "place": "TARGET" } }
        } },
        { "effect": "PLACE", "at": { "place": "TARGET" }, "code": code }
    ] })
}

/// The steps that take a placed or moved stone's liberties into account, for stones of the code
/// `STONE`: the opponent's groups next to `TARGET` that have none are removed, then a group of
/// `TARGET` with none is refused (a suicide).
pub fn liberty_steps() -> Vec<Value> {
    let group = |of: &str| {
        json!({ "region": "GROUP", "of": { "place": of }, "through": "orthogonal",
                "where": { "codes": ["STONE"] } })
    };
    let no_liberty = |of: &str| {
        json!({ "test": "COUNT", "min": 0, "max": 0, "where": { "empty": true },
                "in": { "region": "AROUND", "of": group(of), "through": "orthogonal" } })
    };
    vec![
        json!({ "effect": "FOR_EACH",
                "in": { "region": "AROUND", "of": { "place": "TARGET" }, "through": "orthogonal" },
                "do": [{
                    "if": { "test": "ALL", "of": [
                        { "test": "COUNT", "in": { "region": "AT", "at": { "place": "EACH" } },
                          "where": { "owner": "ENEMY", "codes": ["STONE"] } },
                        no_liberty("EACH")
                    ] },
                    "then": [{ "effect": "FOR_EACH", "in": group("EACH"),
                               "do": [{ "effect": "REMOVE", "at": { "place": "EACH" } }] }]
                }] }),
        json!({ "test": "COUNT", "where": { "empty": true },
                "in": { "region": "AROUND", "of": group("TARGET"), "through": "orthogonal" } }),
    ]
}

/// An `every_action` entry applying `liberty_steps` to the actions `names`.
pub fn liberty_rule(names: &[&str]) -> Value {
    json!({ "for": names, "do": liberty_steps() })
}

/// The action PASS: give up the turn.
pub fn pass_action() -> Value {
    json!({ "name": "PASS", "do": [] })
}

/// The answers given so far this turn.
pub fn chosen(game: &Game) -> Vec<Answer> {
    match &game.state.phase {
        GamePhase::Choosing { chosen } => chosen.clone(),
        GamePhase::GameOver { .. } => Vec::new(),
    }
}

/// Works the turn's ways out again from the board as it stands, when nothing is answered yet.
fn refresh(game: &mut Game) {
    if chosen(game).is_empty() {
        game.work_out_ways();
    }
}

/// Clears the answers given so far, when there are any.
fn cancel(game: &mut Game) -> Result<(), GameError> {
    if chosen(game).is_empty() {
        return Ok(());
    }
    game.transition(GameTransition::Cancel)
}

/// Picks the piece on `at`, putting back a piece picked before. Picking the piece that must
/// keep jumping again changes nothing. A piece of the player to move that has no way to move
/// is picked with nothing to go to, as before: no answer is given, and `destinations` is empty.
pub fn pick(game: &mut Game, at: impl Into<Position>) -> Result<(), GameError> {
    let at = at.into();
    if game.state.continuing.as_ref() == Some(&at) {
        return Ok(());
    }
    cancel(game)?;
    refresh(game);
    let result = game.transition(GameTransition::Choose(Answer::Place(at.clone())));
    let stuck = matches!(game.state.phase, GamePhase::Choosing { .. })
        && game
            .piece_at_position(&at)
            .is_some_and(|piece| piece.player == game.current_player());
    match result {
        Err(GameError::NotAnOption) if stuck => Ok(()),
        other => other,
    }
}

/// Plays the picked piece to `at`.
pub fn go(game: &mut Game, at: impl Into<Position>) -> Result<(), GameError> {
    game.transition(GameTransition::Choose(Answer::Place(at.into())))
}

/// Picks the piece on `from` and plays it to `to`.
pub fn play(
    game: &mut Game,
    from: impl Into<Position>,
    to: impl Into<Position>,
) -> Result<(), GameError> {
    pick(game, from)?;
    go(game, to)
}

/// Answers a pending promotion (any code choice) with `code`.
pub fn promote(game: &mut Game, code: &str) -> Result<(), GameError> {
    game.transition(GameTransition::Choose(Answer::Code(code.to_string())))
}

/// Picks the piece `code` to put on the board: from the hand, or from a supply (an action
/// named after the code). A piece picked before is put back.
pub fn pick_hand(game: &mut Game, code: &str) -> Result<(), GameError> {
    cancel(game)?;
    refresh(game);
    let supplied = ways(game).iter().any(|way| way.action == code);
    let first = if supplied {
        Answer::Action(code.to_string())
    } else {
        Answer::Hand(code.to_string())
    };
    game.transition(GameTransition::Choose(first))
}

/// Puts the piece `code` on `at`: from the hand, or from a supply (an action named after the
/// code). A piece picked before is put back.
pub fn drop(game: &mut Game, code: &str, at: impl Into<Position>) -> Result<(), GameError> {
    pick_hand(game, code)?;
    game.transition(GameTransition::Choose(Answer::Place(at.into())))
}

/// Gives up the turn. A piece picked before is put back.
pub fn pass(game: &mut Game) -> Result<(), GameError> {
    cancel(game)?;
    refresh(game);
    game.transition(GameTransition::Choose(Answer::Action("PASS".to_string())))
}

/// The places the picked piece can go to, by name, in board order.
pub fn destinations(game: &Game) -> Vec<String> {
    targets(game)
        .iter()
        .map(|place| place.to_string())
        .collect()
}

/// The places the picked piece can go to, in board order.
pub fn targets(game: &Game) -> Vec<Position> {
    let mut places: Vec<Position> = game
        .choice()
        .places
        .into_iter()
        .filter(|place| place.r#for == ChoiceKind::Move)
        .map(|place| place.name)
        .collect();
    places.sort();
    places
}

/// Where the piece on `from` can go, picking it (as `pick` does) and reading `targets`.
pub fn moves_from(game: &mut Game, from: impl Into<Position>) -> Vec<Position> {
    pick(game, from).expect("the piece can be picked");
    targets(game)
}

/// The squares the piece `code` can be put on, from the hand or a supply, in board order. The
/// game is left as it was.
pub fn drop_squares(game: &Game, code: &str) -> Vec<Position> {
    let mut game = game.clone();
    if cancel(&mut game).is_err() {
        return Vec::new();
    }
    refresh(&mut game);
    let mut squares: Vec<Position> = ways(&game)
        .iter()
        .filter_map(|way| match way.answers.as_slice() {
            [Answer::Hand(held), Answer::Place(at)] if held == code => Some(at.clone()),
            [Answer::Place(at)] if way.action == code => Some(at.clone()),
            _ => None,
        })
        .collect();
    squares.sort();
    squares.dedup();
    squares
}

/// The turn's ways that the answers so far leave.
pub fn remaining(game: &Game) -> Vec<Way> {
    let given = chosen(game);
    ways(game)
        .iter()
        .filter(|way| answers_match(way, &given))
        .cloned()
        .collect()
}

/// The first remaining way whose next answer is the place `to`: what playing the picked piece
/// to `to` does.
pub fn way_to(game: &Game, to: impl Into<Position>) -> Option<Way> {
    let next = chosen(game)
        .iter()
        .filter(|answer| !matches!(answer, Answer::Action(_)))
        .count();
    let to = Answer::Place(to.into());
    remaining(game)
        .into_iter()
        .find(|way| way.answers.get(next) == Some(&to))
}

/// The codes a pending promotion offers.
pub fn promotion_options(game: &Game) -> Vec<String> {
    game.choice().codes
}

/// Whether the game goes on and the player to move has answered nothing yet.
pub fn phase_is_choosing(game: &Game) -> bool {
    matches!(&game.state.phase, GamePhase::Choosing { chosen } if chosen.is_empty())
}

/// The phase in which the player to move has answered nothing yet.
pub fn choosing() -> GamePhase {
    GamePhase::Choosing { chosen: Vec::new() }
}

fn ways(game: &Game) -> &[Way] {
    game.state.ways.as_deref().unwrap_or_default()
}

fn answers_match(way: &Way, given: &[Answer]) -> bool {
    let mut index = 0;
    given.iter().all(|answer| match answer {
        Answer::Action(name) => *name == way.action,
        answer => {
            index += 1;
            way.answers.get(index - 1) == Some(answer)
        }
    })
}

/// Whether the player to move has a piece among `codes` that the other players attack, on the
/// pieces as they stand: `ATTACKED in BOARD where ALLY codes by ENEMY`, as the end checks of a
/// game with a king write it (`keeping` the codes where a move removes more than its target).
pub fn in_check(game: &Game, codes: &[&str]) -> bool {
    let mut attacked = json!({
        "test": "ATTACKED",
        "in": { "region": "BOARD", "where": { "owner": "ALLY", "codes": codes } },
        "by": "ENEMY"
    });
    if game.rules.removes_beyond_target {
        attacked["keeping"] = json!({ "codes": codes });
    }
    let spec: TestSpec = serde_json::from_value(attacked).expect("the test is read");
    let players: Vec<PlayerSpec> = game
        .rules
        .players
        .iter()
        .map(|name| PlayerSpec::from_name(name))
        .collect();
    let test = Test::build(&spec, &players, None, &mut Labels::default());
    let nowhere = Position::default();
    judge(
        &test,
        &TestContext {
            game,
            pieces: &game.state.pieces,
            player: game.current_player_ref(),
            start: None,
            target: &nowhere,
            each: None,
            removed: &[],
            moved: &[],
            after_turn: None,
        },
    )
}

/// Drops the threefold-repetition end check, to isolate another rule.
pub fn without_repetition(game: &mut Game) {
    Arc::make_mut(&mut game.rules)
        .end_checks
        .retain(|check| !matches!(check.test, Test::Repeats { .. }));
}
