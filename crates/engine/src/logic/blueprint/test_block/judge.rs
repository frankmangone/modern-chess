//! Judging a compiled test on the board.

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use super::{Anchor, Attacker, Filter, Owner, Place, Region, Test, Through};
use crate::logic::{Board, Game, NodeId, Piece};
use crate::shared::Position;

/// What a test is judged against: the game, the position being judged, the mover, and the
/// squares `START`, `TARGET` and `EACH` name (`START` is `None` where there is no moving
/// piece, `EACH` outside a `FOR_EACH`).
pub struct TestContext<'a> {
    pub game: &'a Game,
    /// The pieces as they stand: `&game.state.pieces` until a turn's first effect, then the
    /// position the effects so far have left.
    pub pieces: &'a HashMap<Position, Piece>,
    pub player: &'a str,
    pub start: Option<&'a Position>,
    pub target: &'a Position,
    pub each: Option<&'a Position>,
    /// The pieces the way has taken off the board so far, as they stood; empty where there is
    /// no way being worked out.
    pub removed: &'a [Piece],
}

/// Whether `test` holds. A region that cannot be formed makes `COUNT` and `ATTACKED` false.
pub fn judge(test: &Test, ctx: &TestContext<'_>) -> bool {
    match test {
        Test::All(tests) => tests.iter().all(|test| judge(test, ctx)),
        Test::Any(tests) => tests.iter().any(|test| judge(test, ctx)),
        Test::Not(test) => !judge(test, ctx),
        Test::Count {
            region,
            filter,
            min,
            max,
        } => places(region, ctx).is_some_and(|places| {
            let places = places.as_slice().iter();
            let count = if filter.empty {
                places
                    .filter(|position| !ctx.pieces.contains_key(**position))
                    .count()
            } else {
                places
                    .filter_map(|position| ctx.pieces.get(*position))
                    .filter(|piece| matches(filter, piece, ctx.player))
                    .count()
            };
            usize::from(*min) <= count && count <= usize::from(*max)
        }),
        Test::Attacked { region, by } => {
            places(region, ctx).is_some_and(|places| attacked(places.as_slice(), *by, ctx))
        }
        Test::InZone { at, zone } => {
            locate(at, ctx).is_some_and(|position| ctx.game.in_zone(position, zone, ctx.player))
        }
        Test::Removed(filter) => ctx
            .removed
            .iter()
            .any(|piece| matches(filter, piece, ctx.player)),
        Test::Repeats => ctx.game.position_recurs(ctx.pieces),
    }
}

/// The places of a region: one, without allocating, or several.
enum Places<'a> {
    One(&'a Position),
    Many(Vec<&'a Position>),
}

impl<'a> Places<'a> {
    fn as_slice(&self) -> &[&'a Position] {
        match self {
            Places::One(position) => std::slice::from_ref(position),
            Places::Many(positions) => positions,
        }
    }
}

/// The square `place` names, or `None` when it is off the board (or `START` where there is no
/// moving piece, or `EACH` outside a `FOR_EACH`).
pub(crate) fn locate<'a>(place: &Place, ctx: &TestContext<'a>) -> Option<&'a Position> {
    let anchor = match place.anchor {
        Anchor::Start => ctx.start?,
        Anchor::Target => ctx.target,
        Anchor::Each => ctx.each?,
    };
    match &place.offset {
        None => Some(anchor),
        Some(offsets) => ctx.game.board.step(anchor, *offsets.get(ctx.player)?),
    }
}

/// The places of `region`, in its order; none when it cannot be formed.
pub(crate) fn region_places(region: &Region, ctx: &TestContext<'_>) -> Vec<Position> {
    places(region, ctx).map_or_else(Vec::new, |places| {
        places
            .as_slice()
            .iter()
            .map(|&place| place.clone())
            .collect()
    })
}

/// The places of `region`, or `None` when it cannot be formed.
fn places<'a>(region: &Region, ctx: &TestContext<'a>) -> Option<Places<'a>> {
    let board = &ctx.game.board;
    match region {
        Region::At(at) => locate(at, ctx).map(Places::One),
        Region::Path {
            from,
            to,
            unit,
            from_end,
            to_end,
        } => {
            let from = locate(from, ctx)?;
            let to = locate(to, ctx)?;
            let unit = *unit.as_ref()?.get(ctx.player)?;
            let mut nodes = board.path(board.node(from)?, board.node(to)?, unit)?;
            if nodes.is_empty() {
                return None;
            }
            if !to_end {
                nodes.pop();
            }
            let mut places = Vec::with_capacity(nodes.len() + 1);
            if *from_end {
                places.push(from);
            }
            places.extend(nodes.into_iter().filter_map(|node| board.name(node)));
            Some(Places::Many(places))
        }
        Region::Around { of, through } => {
            let node = board.node(locate(of, ctx)?)?;
            Some(Places::Many(
                neighbours(board, node, *through)
                    .iter()
                    .filter_map(|&n| board.name(n))
                    .collect(),
            ))
        }
        Region::AroundRegion { of, through } => {
            let inner = places(of, ctx)?;
            let inside: Vec<NodeId> = inner
                .as_slice()
                .iter()
                .filter_map(|position| board.node(position))
                .collect();
            let mut met: HashSet<NodeId> = inside.iter().copied().collect();
            let mut around = Vec::new();
            for &node in &inside {
                for &next in neighbours(board, node, *through).iter() {
                    if met.insert(next) {
                        around.extend(board.name(next));
                    }
                }
            }
            Some(Places::Many(around))
        }
        Region::Group {
            of,
            through,
            filter,
        } => Some(Places::Many(group(locate(of, ctx)?, *through, filter, ctx))),
        Region::Line { through, along } => {
            let place = locate(through, ctx)?;
            line(board, place, along.get(ctx.player)?).map(Places::Many)
        }
        Region::Board { filter: None } => Some(Places::Many(board.positions().iter().collect())),
        Region::Board {
            filter: Some(filter),
        } if filter.empty => Some(Places::Many(
            board
                .positions()
                .iter()
                .filter(|position| !ctx.pieces.contains_key(*position))
                .collect(),
        )),
        Region::Board {
            filter: Some(filter),
        } => {
            // The pieces are fewer than the places: find them, then put them in board order.
            let mut held: Vec<(NodeId, &Position)> = ctx
                .pieces
                .iter()
                .filter(|(_, piece)| matches(filter, piece, ctx.player))
                .filter_map(|(position, _)| Some((board.node(position)?, position)))
                .collect();
            held.sort_unstable_by_key(|&(node, _)| node);
            Some(Places::Many(
                held.into_iter().map(|(_, position)| position).collect(),
            ))
        }
    }
}

/// The neighbours of `node` through `through`.
fn neighbours(board: &Board, node: NodeId, through: Through) -> Cow<'_, [NodeId]> {
    match through {
        Through::Adjacent => Cow::Owned(board.adjacent(node)),
        Through::Orthogonal => Cow::Owned(board.orthogonal(node)),
        Through::Linked(label) => Cow::Borrowed(board.linked(node, label)),
    }
}

/// The places of the pieces joined to the piece at `start` through neighbours of its owner
/// that pass `filter`, `start` first; none when `start` holds no piece that passes.
fn group<'a>(
    start: &'a Position,
    through: Through,
    filter: &Filter,
    ctx: &TestContext<'a>,
) -> Vec<&'a Position> {
    let board = &ctx.game.board;
    let passes = |position: &Position| {
        ctx.pieces
            .get(position)
            .filter(|piece| matches(filter, piece, ctx.player))
    };
    let (Some(first), Some(node)) = (passes(start), board.node(start)) else {
        return Vec::new();
    };
    let owner = first.player.as_str();
    let mut members = Vec::new();
    let mut seen = HashSet::from([node]);
    let mut stack = vec![node];
    while let Some(node) = stack.pop() {
        let Some(position) = board.name(node) else {
            continue;
        };
        members.push(position);
        for &next in neighbours(board, node, through).iter() {
            if seen.contains(&next) {
                continue;
            }
            let joins = board
                .name(next)
                .and_then(passes)
                .is_some_and(|piece| piece.player == owner);
            if joins {
                seen.insert(next);
                stack.push(next);
            }
        }
    }
    members
}

/// Every square on the grid's line through `place` along `along`, both ways, `place`
/// included. Disabled squares are left out without ending the line.
fn line<'a>(board: &'a Board, place: &'a Position, along: &[i16]) -> Option<Vec<&'a Position>> {
    let origin = board.coordinates(place)?;
    if origin.len() != along.len() || board.dimensions.len() != along.len() {
        return None;
    }
    let mut places = vec![place];
    if along.iter().all(|&value| value == 0) {
        return Some(places);
    }
    for sign in [1i32, -1] {
        for distance in 1i32.. {
            let square: Option<Vec<u8>> = origin
                .iter()
                .zip(along)
                .zip(&board.dimensions)
                .map(|((&value, &step), &size)| {
                    let moved = i32::from(value) + sign * distance * i32::from(step);
                    u8::try_from(moved).ok().filter(|&moved| moved < size)
                })
                .collect();
            let Some(square) = square else {
                break;
            };
            let named = board
                .node(&Position::from_coordinates(&square))
                .and_then(|node| board.name(node));
            if let Some(position) = named {
                places.push(position);
            }
        }
    }
    Some(places)
}

/// Whether `piece` passes `filter`, seen from `player`.
pub(crate) fn matches(filter: &Filter, piece: &Piece, player: &str) -> bool {
    let owner_matches = match filter.owner {
        Owner::Ally => piece.player == player,
        Owner::Enemy => piece.player != player,
        Owner::Any => true,
    };
    owner_matches
        && (filter.codes.is_empty() || filter.codes.contains(&piece.code))
        && filter
            .flag
            .as_ref()
            .is_none_or(|flag| piece.state.contains_key(flag.as_str()))
        && filter
            .moved
            .is_none_or(|moved| (piece.total_moves > 0) == moved)
}

/// Whether `by` attacks any of `places`, by the attack sets on the position being judged: each
/// opponent's for `Enemy`, the mover's own for `Mover`.
fn attacked(places: &[&Position], by: Attacker, ctx: &TestContext<'_>) -> bool {
    if places.is_empty() {
        return false;
    }
    let game = ctx.game;
    game.rules
        .players
        .iter()
        .filter(|player| match by {
            Attacker::Enemy => player.as_str() != ctx.player,
            Attacker::Mover => player.as_str() == ctx.player,
        })
        .any(|attacker| {
            let attacks =
                Game::attacked_by_pieces(attacker, ctx.pieces, &game.board, &game.rules.blueprints);
            places.iter().any(|position| attacks.contains(*position))
        })
}
