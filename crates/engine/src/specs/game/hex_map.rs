use std::collections::{BTreeMap, BTreeSet, HashSet};

use super::board::{hexagon_corners, HexLayout};
use super::map::{MapSource, MapSpec, PlaceKind, PlaceSpec, MAX_PLACES};
use crate::shared::Coordinates;

const SQRT3: f64 = 1.732_050_807_568_877_2;

/// A tile's `(q, r)`; off-board tiles have `q` or `r` below 0 or past the grid.
type Tile = (i16, i16);

/// A tile's six neighbours, in a cycle: each is next to the one after it, and the last to the
/// first.
const AROUND: [Tile; 6] = [(0, 1), (1, 1), (1, 0), (0, -1), (-1, -1), (-1, 0)];

/// Generates the map of a hex board of `layout` and `size` without the tiles in `disabled`:
/// each enabled cell is an area, each of its six corners a point and each of its six sides a
/// path, all named by the tiles that meet there. Places come tiles, then corners, then sides,
/// each sorted by `(q, r)`. A corner or side exists when it touches at least one tile, so the
/// tiles around it that are cut away or off the grid still name it. Anything in `disabled` that
/// is not a cell of the grid is ignored, as on a hex board.
pub fn hex_map(
    layout: HexLayout,
    size: u8,
    disabled: &HashSet<Coordinates>,
) -> Result<MapSpec, String> {
    let tiles = enabled_tiles(layout, size, disabled)?;
    let rhombus = layout == HexLayout::Rhombus;

    let mut corners: BTreeSet<[Tile; 3]> = BTreeSet::new();
    // A side's ends are the two corners that both of its tiles meet at: for the side between a
    // tile and its neighbour `index`, corners `index − 1` and `index` of the tile.
    let mut sides: BTreeMap<[Tile; 2], [[Tile; 3]; 2]> = BTreeMap::new();
    for &tile in &tiles {
        for index in 0..AROUND.len() {
            let (before, after) = (corner(tile, index + AROUND.len() - 1), corner(tile, index));
            corners.insert(after);
            let mut ends = [before, after];
            ends.sort_unstable();
            sides.insert(side(tile, index), ends);
        }
    }

    let mut places = Vec::with_capacity(tiles.len() + corners.len() + sides.len());
    for &tile in &tiles {
        places.push(PlaceSpec {
            name: tile_name(tile),
            kind: PlaceKind::Area,
            at: [0.0, 0.0],
            ends: None,
            around: (0..AROUND.len())
                .map(|index| meeting_name(&corner(tile, index)))
                .collect(),
            capacity: None,
            start: Default::default(),
        });
    }
    for meeting in &corners {
        places.push(PlaceSpec {
            name: meeting_name(meeting),
            kind: PlaceKind::Point,
            at: average(meeting.iter().map(|&tile| centre(rhombus, tile))),
            ends: None,
            around: Vec::new(),
            capacity: None,
            start: Default::default(),
        });
    }
    for (meeting, ends) in &sides {
        places.push(PlaceSpec {
            name: meeting_name(meeting),
            kind: PlaceKind::Path,
            at: [0.0, 0.0],
            ends: Some(ends.map(|end| meeting_name(&end))),
            around: Vec::new(),
            capacity: None,
            start: Default::default(),
        });
    }

    let source = MapSource::HexMap {
        layout,
        size,
        disabled: disabled.clone(),
    };
    MapSpec::compile(places, Vec::new(), Vec::new(), source)
}

/// The enabled cells of the grid a hex board of `layout` and `size` has, sorted.
fn enabled_tiles(
    layout: HexLayout,
    size: u8,
    disabled: &HashSet<Coordinates>,
) -> Result<Vec<Tile>, String> {
    if size == 0 {
        return Err("a hex map needs a `size` of at least 1".into());
    }
    let (side, cut) = match layout {
        HexLayout::Rhombus => (size, HashSet::new()),
        HexLayout::Hexagon => {
            let side = size
                .checked_mul(2)
                .and_then(|doubled| doubled.checked_sub(1))
                .ok_or("the hexagon is too large")?;
            (side, hexagon_corners(side, size))
        }
    };
    let mut tiles = Vec::new();
    for q in 0..side {
        for r in 0..side {
            let cell = vec![q, r];
            if cut.contains(&cell) || disabled.contains(&cell) {
                continue;
            }
            tiles.push((i16::from(q), i16::from(r)));
            if tiles.len() > MAX_PLACES {
                return Err(format!("the map has more than {MAX_PLACES} places"));
            }
        }
    }
    Ok(tiles)
}

/// The tile `index` places round the cycle from `tile`.
fn neighbour(tile: Tile, index: usize) -> Tile {
    let (dq, dr) = AROUND.get(index % AROUND.len()).copied().unwrap_or((0, 0));
    (tile.0 + dq, tile.1 + dr)
}

/// The three tiles that meet at corner `index` of `tile`: it and the neighbours `index` and
/// `index + 1`.
fn corner(tile: Tile, index: usize) -> [Tile; 3] {
    let mut meeting = [tile, neighbour(tile, index), neighbour(tile, index + 1)];
    meeting.sort_unstable();
    meeting
}

/// The two tiles a side lies between: `tile` and its neighbour `index`.
fn side(tile: Tile, index: usize) -> [Tile; 2] {
    let mut meeting = [tile, neighbour(tile, index)];
    meeting.sort_unstable();
    meeting
}

fn tile_name((q, r): Tile) -> String {
    format!("{q},{r}")
}

/// The tiles joined by `|`.
fn meeting_name(tiles: &[Tile]) -> String {
    tiles
        .iter()
        .map(|&tile| tile_name(tile))
        .collect::<Vec<_>>()
        .join("|")
}

/// Where a tile is drawn, as a hex board draws it (`hexPoint` in the docs' board model): a
/// rhombus is turned by −120°.
fn centre(rhombus: bool, (q, r): Tile) -> [f64; 2] {
    let (q, r) = (f64::from(q), f64::from(r));
    let (x, y) = (1.5 * q, SQRT3 * (r - q / 2.0));
    if rhombus {
        [-0.5 * x + SQRT3 / 2.0 * y, -SQRT3 / 2.0 * x - 0.5 * y]
    } else {
        [x, y]
    }
}

fn average(points: impl Iterator<Item = [f64; 2]>) -> [f64; 2] {
    let (count, x, y) = points.fold((0.0, 0.0, 0.0), |(n, x, y), at| {
        (n + 1.0, x + at[0], y + at[1])
    });
    [x / count, y / count]
}
