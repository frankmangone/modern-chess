// The parts every game is made of that are not rules: the board's places and links, players,
// turns, pieces, bags, zones, end checks and the sections of a page. Plain functions that return
// plain data, keys in the order the committed files have them.
import type { Test, Where } from "./blocks.ts";

// ---------- Board ----------

export interface MapPlace {
  name: string;
  at: [number, number];
  capacity: number;
  /** How many of each piece stands here as the game begins. */
  start?: Record<string, number>;
}

export type MapLink =
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
    const ends = "between" in each ? each.between : [each.from, each.to];
    for (const end of ends) if (!names.has(end)) throw new Error(`mapBoard: a link names ${end}, which is not a place`);
  }
  return { shape: "map", places, links };
}

// ---------- Players, turns, pieces, bags, zones ----------

export interface PlayerDef {
  name: string;
  starting_positions: never[];
}

/** Players whose pieces start where the board's places say, so they list none of their own. */
export function players(...names: string[]): PlayerDef[] {
  if (names.length === 0) throw new Error("players: at least one player is needed");
  return names.map((name) => ({ name, starting_positions: [] }));
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
  moves: never[];
}

/** A piece that moves only by what the actions say. Its name is its code in lower case. */
export function piece(code: string, name: string = code.toLowerCase().replaceAll("_", " ")): PieceDef {
  if (code.length === 0) throw new Error("piece: a code is needed");
  return { code, name, moves: [] };
}

export interface BagDef {
  name: string;
  for?: "EACH_PLAYER";
  capacity?: number;
}

/** A bag nobody owns, or with `eachPlayer` one for every player. */
export function bagDef(name: string, options: { eachPlayer?: boolean; capacity?: number } = {}): BagDef {
  if (name.length === 0) throw new Error("bagDef: a name is needed");
  const def: BagDef = { name };
  if (options.eachPlayer) def.for = "EACH_PLAYER";
  if (options.capacity !== undefined) def.capacity = options.capacity;
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

export interface ScoreTerm {
  term: string;
  zone: string;
  where: Where;
}

export function scoreTerm(term: string, zoneName: string, where: Where): ScoreTerm {
  return { term, zone: zoneName, where };
}

// ---------- Pages ----------

export type Section = { type: "how_to_play"; markdown: string } | { type: "preview" } | { type: "open_lobbies" };

/** The sections a game's page has: its rules in words, the board to try, and the lobbies. */
export function standardSections(markdown: string): Section[] {
  if (markdown.trim().length === 0) throw new Error("standardSections: the rules need words");
  return [{ type: "how_to_play", markdown }, { type: "preview" }, { type: "open_lobbies" }];
}
