// The parts every game is made of that are not rules: the board's places and links, players,
// turns, pieces, bags, zones, end checks and the sections of a page. Plain functions that return
// plain data, keys in the order the committed files have them.
import { count, moveHeld, not, place, atPlace, TARGET, type Test, type Where } from "./blocks.ts";

// ---------- Board ----------

export interface MapPlace {
  name: string;
  at: [number, number];
  capacity: number;
  /** How many of each piece stands here as the game begins. */
  start?: Record<string, number>;
}

export type MapLink =
  | [string, string]
  | { from: string; to: string; label?: string }
  | { between: [string, string]; label: string };

export interface MapBoard {
  shape: "map";
  places: MapPlace[];
  links: MapLink[];
}

/** A place of a map board. `start` is left out when nothing starts there. */
export function mapPlace(name: string, at: [number, number], capacity: number, start?: Record<string, number>): MapPlace {
  if (name.length === 0) throw new Error("mapPlace: a name is needed");
  if (!Number.isInteger(capacity) || capacity < 1) throw new Error(`mapPlace ${name}: the capacity must be 1 or more (got ${capacity})`);
  const place: MapPlace = { name, at, capacity };
  if (start !== undefined && Object.keys(start).length > 0) place.start = start;
  return place;
}

/** A one-way link; with a label, a labelled one. */
export function link(from: string, to: string, label?: string): MapLink {
  return label === undefined ? { from, to } : { from, to, label };
}

/** A link with no label that goes both ways, written as the pair, as the engine reads it. */
export function bothWays(a: string, b: string): MapLink {
  if (a.length === 0 || b.length === 0) throw new Error("bothWays: both ends need a name");
  return [a, b];
}

/** A labelled link that goes both ways. */
export function between(a: string, b: string, label: string): MapLink {
  if (label.length === 0) throw new Error("between: a label is needed");
  return { between: [a, b], label };
}

export function mapBoard(places: MapPlace[], links: MapLink[]): MapBoard {
  const names = new Set<string>();
  for (const place of places) {
    if (names.has(place.name)) throw new Error(`mapBoard: two places are named ${place.name}`);
    names.add(place.name);
  }
  for (const each of links) {
    const ends = Array.isArray(each) ? each : "between" in each ? each.between : [each.from, each.to];
    for (const end of ends) if (!names.has(end)) throw new Error(`mapBoard: a link names ${end}, which is not a place`);
  }
  return { shape: "map", places, links };
}

// ---------- Players, turns, pieces, bags, zones ----------

export interface PlayerDef {
  name: string;
  direction?: number[][];
  starting_positions: { piece: string; positions: string[] }[];
}

/** Players whose pieces start where the board's places say, so they list none of their own. */
export function players(...names: string[]): PlayerDef[] {
  if (names.length === 0) throw new Error("players: at least one player is needed");
  return names.map((name) => ({ name, starting_positions: [] }));
}

/** Players whose pieces start on named places of a map: `{ WHITE: ["a", "b"], BLACK: [...] }`. A
 *  player with no places starts with nothing on the board. */
export function playersAt(piece: string, at: Record<string, string[]>): PlayerDef[] {
  const entries = Object.entries(at);
  if (entries.length === 0) throw new Error("playersAt: at least one player is needed");
  return entries.map(([name, positions]) => ({ name, starting_positions: positions.length === 0 ? [] : [{ piece, positions }] }));
}

/** Two players on a grid board, who need a `direction` though nothing on the board moves: the
 *  first faces up the board and the second down it. */
export function gridPlayers(first: string, second: string): PlayerDef[] {
  return [
    { name: first, direction: [[1, 0], [0, 1]], starting_positions: [] },
    { name: second, direction: [[-1, 0], [0, -1]], starting_positions: [] },
  ];
}

export interface TurnsDef {
  order: string[];
  stages?: { name: string }[];
}

export function turns(order: string[], stages?: string[]): TurnsDef {
  if (order.length === 0) throw new Error("turns: the order needs a player");
  return stages === undefined ? { order } : { order, stages: stages.map((name) => ({ name })) };
}

export interface PieceDef {
  code: string;
  name: string;
  moves: unknown[];
}

/** A piece that moves only by what the actions say. Its name is its code in lower case. */
export function piece(code: string, name: string = code.toLowerCase().replaceAll("_", " ")): PieceDef {
  if (code.length === 0) throw new Error("piece: a code is needed");
  return { code, name, moves: [] };
}

/** A piece that slides along one link of a map to an empty place, and does nothing else. */
export function slider(code: string, name: string = code.toLowerCase().replaceAll("_", " ")): PieceDef {
  if (code.length === 0) throw new Error("slider: a code is needed");
  return { code, name, moves: [{ id: 0, step: "next", do: [not(count(atPlace(TARGET))), moveHeld(place(TARGET))] }] };
}

export interface BagDef {
  name: string;
  for?: "EACH_PLAYER";
  capacity?: number;
  start?: Record<string, number>;
  ordered?: boolean;
  visible?: "ALL" | "OWNER" | "NONE";
  shows_count?: boolean;
  strict?: boolean;
}

export interface BagOptions {
  eachPlayer?: boolean;
  capacity?: number;
  /** What the bag holds as the game begins: `{ code: count }`. Every player's, for `eachPlayer`. */
  start?: Record<string, number>;
  /** A pile: it keeps its pieces in order, and can be shuffled. */
  ordered?: boolean;
  /** Who sees the pieces: everyone (the default), only the owner, or nobody. */
  visible?: "ALL" | "OWNER" | "NONE";
  /** Whether everyone is told how many pieces a hidden bag holds (the default is yes). */
  showsCount?: boolean;
  /** Refuses any rule that would read what the bag hides from the player it is judged for. */
  strict?: boolean;
}

/** A bag nobody owns, or with `eachPlayer` one for every player. Keys are written in a fixed
 *  order, and only the ones given. */
export function bagDef(name: string, options: BagOptions = {}): BagDef {
  if (name.length === 0) throw new Error("bagDef: a name is needed");
  if (options.visible === "OWNER" && !options.eachPlayer) throw new Error(`bagDef ${name}: only a bag each player has can be visible to its owner`);
  const def: BagDef = { name };
  if (options.eachPlayer) def.for = "EACH_PLAYER";
  if (options.capacity !== undefined) def.capacity = options.capacity;
  if (options.start !== undefined) def.start = options.start;
  if (options.ordered) def.ordered = true;
  if (options.visible !== undefined) def.visible = options.visible;
  if (options.showsCount !== undefined) def.shows_count = options.showsCount;
  if (options.strict) def.strict = true;
  return def;
}

export interface ZoneDef {
  name: string;
  places: Record<string, string[]>;
}

/** A named set of places, listed for each player. */
export function zone(name: string, places: Record<string, string[]>): ZoneDef {
  if (name.length === 0) throw new Error("zone: a name is needed");
  return { name, places };
}

export interface GridBoard {
  dimensions: [number, number];
}

/** A grid board, for a game that hardly uses one. */
export function gridBoard(columns: number, rows: number): GridBoard {
  if (!Number.isInteger(columns) || !Number.isInteger(rows) || columns < 1 || rows < 1) throw new Error(`gridBoard: the size must be whole numbers of 1 or more (got ${columns} by ${rows})`);
  return { dimensions: [columns, rows] };
}

// ---------- Ending ----------

export interface EndCheck {
  as?: string;
  when: Test;
  then: { outcome: string };
}

/** A game ends with an outcome (`WIN`, `HIGHEST`, ...) when the test holds, judged from the
 *  point of view of `as` (`NEXT`: the player who would play next) when given. */
export function endCheck(when: Test, outcome: string, options: { as?: string } = {}): EndCheck {
  const check: EndCheck = options.as === undefined ? ({ when, then: { outcome } } as EndCheck) : { as: options.as, when, then: { outcome } };
  return check;
}

export type ScoreTerm = { term: string; zone: string; where: Where } | { term: "BAGGED"; bag: string; where: Where };

export function scoreTerm(term: string, zoneName: string, where: Where): ScoreTerm {
  return { term, zone: zoneName, where };
}

/** A point for each piece of the given kind in the player's own bag. A card worth several points
 *  is written once for each point (`GOLD` three times). */
export function baggedTerm(bagName: string, where: Where): ScoreTerm {
  return { term: "BAGGED", bag: bagName, where };
}

// ---------- Pages ----------

export type Section = { type: "how_to_play"; markdown: string } | { type: "preview" } | { type: "open_lobbies" };

/** The sections a game's page has: its rules in words, the board to try, and the lobbies. */
export function standardSections(markdown: string): Section[] {
  if (markdown.trim().length === 0) throw new Error("standardSections: the rules need words");
  return [{ type: "how_to_play", markdown }, { type: "preview" }, { type: "open_lobbies" }];
}
