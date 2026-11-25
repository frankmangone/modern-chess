// Pure helpers for the board: skins, fallback pieces and geometry.
// Nothing here knows the rules of any game.

const DEFAULT_SQUARES = { light: "#f0d9b5", dark: "#b58863" };
const PLAYER_COLOURS = ["#d94f4f", "#3f6fd9", "#3fa86b", "#d9a13f"];
const NEUTRAL_COLOUR = "#777777";

const isObject = (v) => typeof v === "object" && v !== null && !Array.isArray(v);

// An image address a skin may use: a path on this site, in plain characters, to a picture file.
// Nothing that names another site, runs code or climbs out of the folder gets through.
const IMAGE_PATH = /^\/(?!\/)[A-Za-z0-9_\-./]+\.(svg|png|webp|jpe?g)$/;
const isImagePath = (value) => typeof value === "string" && IMAGE_PATH.test(value) && !value.includes("..");

// A colour a skin may use: a hex value, a colour name or rgb()/hsl() with plain numbers. Nothing
// that loads a file, names another style rule or runs code gets through.
const HEX_COLOUR = /^#[0-9a-fA-F]{3,8}$/;
const NAMED_COLOUR = /^[a-zA-Z]{3,20}$/;
const FUNCTION_COLOUR = /^(rgb|rgba|hsl|hsla)\(\s*[0-9.,%\s/-]+\)$/;
const isColour = (value) =>
  typeof value === "string" && (HEX_COLOUR.test(value) || NAMED_COLOUR.test(value) || FUNCTION_COLOUR.test(value));

// Board drawings are numbers in intersection units: whole numbers for crossings, halves for the
// space between two. A skin cannot ask for a large drawing.
const MAX_ITEMS = 200;
const isCoordinate = (value) => Number.isFinite(value) && Math.abs(value) <= 100;
const isPoint = (value) => Array.isArray(value) && value.length === 2 && value.every(isCoordinate);

function listOf(raw, accept) {
  return Array.isArray(raw) ? raw.filter(accept).slice(0, MAX_ITEMS) : [];
}

const BOARD_STYLES = ["squares", "intersections"];

function normalizeBoard(raw) {
  const board = {
    style: "squares",
    background: null,
    lines: null,
    breaks: [],
    diagonals: [],
    dots: [],
    crosses: [],
    labels: [],
  };
  if (!isObject(raw)) return board;
  if (BOARD_STYLES.includes(raw.style)) board.style = raw.style;
  if (isColour(raw.background)) board.background = raw.background;
  if (isColour(raw.lines)) board.lines = raw.lines;
  // Rows where the lines between two neighbouring rows stop, but for the two edge files (a river).
  board.breaks = listOf(raw.breaks, isPoint);
  board.diagonals = listOf(raw.diagonals, (d) => Array.isArray(d) && d.length === 2 && d.every(isPoint));
  board.dots = listOf(raw.dots, isPoint);
  board.crosses = listOf(raw.crosses, isPoint);
  board.labels = listOf(
    raw.labels,
    (l) => isObject(l) && typeof l.text === "string" && l.text.length > 0 && l.text.length <= 12 && isCoordinate(l.x) && isCoordinate(l.y),
  ).map(({ text, x, y }) => ({ text, x, y }));
  return board;
}

// A piece face: a glyph string, { svg: "<svg…>" } or { image: "/path/to/file.svg" }. Anything
// else is dropped.
function normalizeFace(raw) {
  if (typeof raw === "string" && raw.length > 0) return { kind: "glyph", value: raw };
  if (isObject(raw) && typeof raw.svg === "string" && raw.svg.length > 0) {
    return { kind: "svg", value: raw.svg };
  }
  if (isObject(raw) && isImagePath(raw.image)) return { kind: "image", value: raw.image };
  return null;
}

function normalizePieceEntry(raw) {
  const shared = normalizeFace(raw);
  if (shared) return { shared };
  if (!isObject(raw)) return null;
  const perPlayer = {};
  for (const [player, face] of Object.entries(raw)) {
    const normalized = normalizeFace(face);
    if (normalized) perPlayer[player] = normalized;
  }
  return Object.keys(perPlayer).length > 0 ? { perPlayer } : null;
}

/** Turns anything into a usable skin. Never throws. */
export function normalizeSkin(raw) {
  const skin = {
    pieces: {},
    squares: { ...DEFAULT_SQUARES },
    zones: {},
    facing: false,
    board: normalizeBoard(undefined),
    colours: {},
    pieceStyle: "plain",
  };
  if (!isObject(raw)) return skin;

  if (isObject(raw.pieces)) {
    for (const [code, entry] of Object.entries(raw.pieces)) {
      const normalized = normalizePieceEntry(entry);
      if (normalized) skin.pieces[code] = normalized;
    }
  }
  if (isObject(raw.squares)) {
    if (typeof raw.squares.light === "string") skin.squares.light = raw.squares.light;
    if (typeof raw.squares.dark === "string") skin.squares.dark = raw.squares.dark;
  }
  // Hex boards colour their cells in three: a skin that gives two gets the colour between them.
  skin.squares.mid =
    isObject(raw.squares) && isColour(raw.squares.mid)
      ? raw.squares.mid
      : mixColours(skin.squares.light, skin.squares.dark);
  skin.facing = raw.facing === true;
  skin.board = normalizeBoard(raw.board);
  if (isObject(raw.colours)) {
    for (const [player, colour] of Object.entries(raw.colours)) {
      if (isColour(colour)) skin.colours[player] = colour;
    }
  }
  if (raw.pieceStyle === "disc") skin.pieceStyle = "disc";
  if (isObject(raw.zones)) {
    for (const [name, colour] of Object.entries(raw.zones)) {
      if (typeof colour === "string") skin.zones[name] = colour;
      // One colour per player: each player's cells of the zone are tinted in theirs.
      if (isObject(colour)) {
        const perPlayer = Object.fromEntries(Object.entries(colour).filter(([, c]) => isColour(c)));
        if (Object.keys(perPlayer).length > 0) skin.zones[name] = perPlayer;
      }
    }
  }
  return skin;
}

function playerColour(skin, player, players) {
  if (skin.colours[player]) return skin.colours[player];
  const index = players.indexOf(player);
  return index >= 0 ? PLAYER_COLOURS[index % PLAYER_COLOURS.length] : NEUTRAL_COLOUR;
}

/**
 * What to draw for a piece: { kind: "glyph" | "svg" | "image" | "fallback", value, colour?, disc? }.
 * A per-player entry wins; a single entry is tinted by owner; otherwise the piece code's first
 * letter, coloured by owner. With the disc style, text and drawings sit on a round tile.
 */
export function pieceFace(skin, code, player, players) {
  const entry = skin.pieces[code];
  let face;
  if (entry?.perPlayer?.[player]) {
    face = { ...entry.perPlayer[player] };
    // A colour chosen for the player is kept; otherwise the face is drawn as it is written.
    if (skin.colours[player] && face.kind !== "image") face.colour = skin.colours[player];
  } else if (entry?.shared) {
    // A picture is drawn as it is. Only text and drawings take the owner's colour.
    face = { ...entry.shared };
    if (face.kind !== "image") face.colour = playerColour(skin, player, players);
  } else {
    face = {
      kind: "fallback",
      value: (code[0] ?? "?").toUpperCase(),
      colour: playerColour(skin, player, players),
    };
  }
  if (skin.pieceStyle === "disc" && face.kind !== "image") face.disc = true;
  return face;
}

/**
 * Whether a piece is drawn turned half way round. With the skin's `facing` option on, the player
 * at the top of the board has pieces that point down at the opponent, as on a real Shogi board.
 * The player at the top is the second one, or the first one when the board is flipped.
 */
export function isTurned(skin, player, players, flipped) {
  if (!skin.facing || players.length !== 2) return false;
  return player === players[flipped ? 0 : 1];
}

/** The tint colour for a node. Zones coloured per player come first: the node takes the
 *  colour of the first player (in turn order) whose cells of any such zone contain it, so a
 *  corner two players share has one owner. Otherwise it takes the colour of the first
 *  one-colour zone that contains it. `name` is the node's name. */
export function zoneTint(skin, view, name) {
  const holds = (names) => (names ?? []).includes(name);
  const zones = Object.entries(skin.zones).filter(([zone]) => view.zones?.[zone]);
  const players = view.players ?? [];
  for (const player of players) {
    for (const [zone, colour] of zones) {
      if (typeof colour !== "string" && colour[player] && holds(view.zones[zone][player])) return colour[player];
    }
  }
  for (const [zone, colour] of zones) {
    if (typeof colour === "string" && Object.values(view.zones[zone]).some(holds)) return colour;
  }
  return null;
}

/** Every zone colour that applies to a node, as the skin writes it: per-player zones first (in
 *  turn order), then one-colour zones, without repeats. */
export function zoneColours(skin, view, name) {
  const holds = (names) => (names ?? []).includes(name);
  const zones = Object.entries(skin.zones).filter(([zone]) => view.zones?.[zone]);
  const found = [];
  for (const player of view.players ?? []) {
    for (const [zone, colour] of zones) {
      if (typeof colour !== "string" && colour[player] && holds(view.zones[zone][player])) found.push(colour[player]);
    }
  }
  for (const [zone, colour] of zones) {
    if (typeof colour === "string" && Object.values(view.zones[zone]).some(holds)) found.push(colour);
  }
  return [...new Set(found)];
}

/** The zone colours of a node made strong enough to read as stripes. */
export function zoneStripeColours(skin, view, name) {
  return zoneColours(skin, view, name).map(stripeColour);
}

/** A tint made into a stripe colour: a `#rgb` / `#rrggbb` is darkened by a third and made mostly
 *  opaque, so a pale wash still shows on light and dark squares. Anything else is used as written. */
export function stripeColour(colour) {
  const hex = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(colour)?.[1];
  if (!hex) return colour;
  const full = hex.length === 3 ? [...hex].map((c) => c + c).join("") : hex;
  const [r, g, b] = [0, 2, 4].map((i) => Math.round(parseInt(full.slice(i, i + 2), 16) * 0.65));
  return `rgba(${r}, ${g}, ${b}, 0.8)`;
}

/** How strongly a zone colour washes the cell under its stripes. */
export const ZONE_WASH = 0.3;

/** Which way each zone colour's stripes run, in order: the first as the hero's (-45°), the second
 *  across it, so a cell two players share is cross-hatched rather than muddled. */
const STRIPE_ANGLES = [-45, 45, 0, 90];

export function stripeAngle(index) {
  return STRIPE_ANGLES[index % STRIPE_ANGLES.length];
}

/** The zone mark for a cell, from its zone colours as the skin writes them: each colour's 3px
 *  stripes in a direction of its own, over a light wash of every colour. A cell in two players'
 *  zones is cross-hatched, and its washes mix (red and blue make a purple). */
export function stripeGradient(colours) {
  const stripes = colours.map(
    (colour, i) => `repeating-linear-gradient(${stripeAngle(i)}deg, ${stripeColour(colour)} 0 3px, transparent 3px 8px)`,
  );
  const washes = colours.map((colour) => {
    const wash = `color-mix(in srgb, ${colour} ${Math.round(ZONE_WASH * 100)}%, transparent)`;
    return `linear-gradient(${wash}, ${wash})`;
  });
  return [...stripes, ...washes].join(", ");
}

/** The colour halfway between two `#rgb` / `#rrggbb` colours; the first one when either is
 *  written another way. */
function mixColours(a, b) {
  const channels = (colour) => {
    const hex = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(colour)?.[1];
    if (!hex) return null;
    const full = hex.length === 3 ? [...hex].map((c) => c + c).join("") : hex;
    return [0, 2, 4].map((i) => parseInt(full.slice(i, i + 2), 16));
  };
  const [ca, cb] = [channels(a), channels(b)];
  if (!ca || !cb) return a;
  return "#" + ca.map((v, i) => Math.round((v + cb[i]) / 2).toString(16).padStart(2, "0")).join("");
}

// ----- Grids -----
//
// The view lists a board's nodes: each has a `name`, which is what a click answers with, and a
// place to draw it, `at: [x, y]`. A cell the board does not have is not listed.

/** The smallest box, in drawing places, that holds every node. */
export function gridBounds(view) {
  const xs = view.nodes.map(({ at }) => at[0]);
  const ys = view.nodes.map(({ at }) => at[1]);
  return { minX: Math.min(...xs), minY: Math.min(...ys), maxX: Math.max(...xs), maxY: Math.max(...ys) };
}

const placeKey = (x, y) => `${x},${y}`;

/** Whether a square board has holes: a place inside its bounds that is not on it (a cross). */
export function hasHoles(view) {
  const { minX, minY, maxX, maxY } = gridBounds(view);
  return view.nodes.length < (maxX - minX + 1) * (maxY - minY + 1);
}

/** The lines of a board with holes, as pairs of places: one between each two neighbours (along a
 *  row or a column) that are both on the board, so no line crosses a hole. */
export function neighbourLines(view) {
  const on = new Set(view.nodes.map(({ at }) => placeKey(at[0], at[1])));
  const lines = [];
  for (const { at } of view.nodes) {
    const [x, y] = at;
    if (on.has(placeKey(x + 1, y))) lines.push([[x, y], [x + 1, y]]);
    if (on.has(placeKey(x, y + 1))) lines.push([[x, y], [x, y + 1]]);
  }
  return lines;
}

/** The rows of a square board, top first, each as wide as the board: the node drawn at that
 *  place, or null where there is none (a hole). The first row of the board ends up at the
 *  bottom; a flipped board is that picture turned half way round. */
export function gridRows(view, flipped) {
  const { minX, minY, maxX, maxY } = gridBounds(view);
  const nodes = new Map(view.nodes.map((node) => [placeKey(...node.at), node]));
  const rows = [];
  for (let row = 0; row <= maxY - minY; row++) {
    const y = flipped ? minY + row : maxY - row;
    const cells = [];
    for (let column = 0; column <= maxX - minX; column++) {
      const x = flipped ? maxX - column : minX + column;
      cells.push(nodes.get(placeKey(x, y)) ?? null);
    }
    rows.push(cells);
  }
  return rows;
}

// ----- Hex boards -----
//
// A hex node drawn at (x, y) is drawn flat-topped, in units of the hex's radius with y up, at
// (1.5·x, √3·(y − x/2)). An upright hexagon board uses that as it is; a rhombus is turned so its
// long diagonal, from (n−1, 0) to (0, n−1), lies horizontal: the traditional wide diamond. A hex
// turned by 120° looks the same, so its cells stay flat-topped.

const SQRT3 = Math.sqrt(3);

function hexPoint(view, x, y) {
  const px = 1.5 * x;
  const py = SQRT3 * (y - x / 2);
  if (view.layout !== "rhombus") return [px, py];
  // Turn by −120°: cos = −1/2, sin = −√3/2.
  return [-0.5 * px + (SQRT3 / 2) * py, -(SQRT3 / 2) * px - 0.5 * py];
}

/** The board's box: `aspect` is its width over its height, and `cellWidth` / `cellHeight` are
 *  one hex's size as a share of the box's width and height. `bounds` is the nodes' `gridBounds`. */
export function hexGeometry(view) {
  const points = view.nodes.map(({ at }) => hexPoint(view, at[0], at[1]));
  const xs = points.map(([px]) => px);
  const ys = points.map(([, py]) => py);
  const minX = Math.min(...xs) - 1;
  const maxX = Math.max(...xs) + 1;
  const minY = Math.min(...ys) - SQRT3 / 2;
  const maxY = Math.max(...ys) + SQRT3 / 2;
  const width = maxX - minX;
  const height = maxY - minY;
  return {
    minX,
    maxY,
    width,
    height,
    aspect: width / height,
    cellWidth: 2 / width,
    cellHeight: SQRT3 / height,
    bounds: gridBounds(view),
  };
}

/** Where the centre of a node drawn at `at` is placed, as shares of the board box (0 to 1, from
 *  the top left). A flipped board is turned half way round, within the nodes' bounds. Pass the
 *  board's `hexGeometry` when placing many nodes, so it is worked out once. */
export function hexCentre(view, at, flipped = false, box = hexGeometry(view)) {
  const { minX, minY, maxX, maxY } = box.bounds;
  const [x, y] = flipped ? [minX + maxX - at[0], minY + maxY - at[1]] : at;
  const [px, py] = hexPoint(view, x, y);
  return { left: (px - box.minX) / box.width, top: (box.maxY - py) / box.height };
}

/** Which of a hex board's three colours a cell takes: 0 light, 1 mid, 2 dark. No two neighbours
 *  share one, and a bishop's diagonal keeps it. */
export function hexColour(x, y) {
  return (x + y) % 3;
}

// ----- Maps -----
//
// A map's places are drawn where their `at` says, in the map's own units with y up. A map is
// drawn as large as a grid of the same spacing: one `unit`, the shortest distance between two
// points, is the size of one cell. Paths and areas are drawn at their `at` too (the engine sends a
// path's midpoint and an area's centre), so a place of any kind is placed the same way.

/** The smallest box that holds every place, and `unit`: the shortest distance between two points
 *  (1 when the map has fewer than two, or its points all share one spot). */
export function mapGeometry(view) {
  const xs = view.nodes.map(({ at }) => at[0]);
  const ys = view.nodes.map(({ at }) => at[1]);
  return {
    minX: Math.min(...xs),
    minY: Math.min(...ys),
    maxX: Math.max(...xs),
    maxY: Math.max(...ys),
    unit: shortestGap(view.nodes.filter(({ kind }) => (kind ?? "point") === "point")),
  };
}

// Sweeps the points left to right and stops looking past the best gap so far, so a map of
// thousands of points is not compared pair by pair.
function shortestGap(points) {
  const sorted = points.map(({ at }) => at).sort((a, b) => a[0] - b[0]);
  let best = Infinity;
  for (let i = 0; i < sorted.length; i++) {
    for (let j = i + 1; j < sorted.length && sorted[j][0] - sorted[i][0] < best; j++) {
      const gap = Math.hypot(sorted[j][0] - sorted[i][0], sorted[j][1] - sorted[i][1]);
      if (gap > 0 && gap < best) best = gap;
    }
  }
  return Number.isFinite(best) ? best : 1;
}

/** The board box's width and height in map units: the bounds with one unit of padding all round. */
export function mapSize({ minX, minY, maxX, maxY, unit }) {
  return { width: maxX - minX + 2 * unit, height: maxY - minY + 2 * unit };
}

/** Where a place drawn at `at` is, as shares of the board box (0 to 1, from the top left). A
 *  flipped map is turned half way round, within the places' bounds. Pass the map's `mapGeometry`
 *  when placing many places, so it is worked out once. */
export function mapPlace(view, at, flipped = false, geometry = mapGeometry(view)) {
  const { minX, minY, maxX, maxY, unit } = geometry;
  const [x, y] = flipped ? [minX + maxX - at[0], minY + maxY - at[1]] : at;
  const { width, height } = mapSize(geometry);
  return { left: (x - minX + unit) / width, top: (maxY - y + unit) / height };
}

// The pieces of a view by place. A place that holds many can list several entries (one per owner
// and code), so each place maps to all of its entries, in the order the view gave them.
export function groupPieces(pieces) {
  const grouped = new Map();
  for (const piece of pieces) {
    const entries = grouped.get(piece.position);
    if (entries) entries.push(piece);
    else grouped.set(piece.position, [piece]);
  }
  return grouped;
}

// What a screen reader hears for a place with pieces on it: every entry, then where.
export function piecesLabel(entries, position) {
  const named = entries.map((piece) => {
    const count = piece.count > 1 ? ` ×${piece.count}` : "";
    return `${piece.player || "neutral"} ${piece.code}${count}`;
  });
  return `${named.join(", ")} at ${position}`;
}
