// A numbered track of squares, and the jumps between them (ladders and snakes).
import { link, mapPlace, type MapLink, type MapPlace } from "../scaffold.ts";

/** Where square `n` (counted from 1) lies on a track that runs along its first row from the left,
 *  then back along the next from the right, and so on up: [column, row], row 0 at the bottom. */
export function squareAt(n: number, rowLength: number): [number, number] {
  if (!Number.isInteger(n) || n < 1) throw new Error(`squareAt: squares count from 1 (got ${n})`);
  const row = Math.floor((n - 1) / rowLength);
  let column = (n - 1) % rowLength;
  if (row % 2 === 1) column = rowLength - 1 - column;
  return [column, row];
}

export interface TrackOptions {
  /** The number of the last square. The squares are named "1" to this. */
  squares: number;
  rowLength: number;
  /** How many pieces fit on a square, the start pad included. */
  capacity: number;
  /** A pad off the board where pieces wait before square 1, named "0". */
  pad: { at: [number, number]; start: Record<string, number> };
}

/** The pad, then every square. */
export function trackPlaces(options: TrackOptions): MapPlace[] {
  const { squares, rowLength, capacity, pad } = options;
  const places = [mapPlace("0", pad.at, capacity, pad.start)];
  for (let n = 1; n <= squares; n += 1) places.push(mapPlace(String(n), squareAt(n, rowLength), capacity));
  return places;
}

/** One-way links from each square to the next, from the pad to the last square. */
export function stepLinks(squares: number, label: string): MapLink[] {
  const links: MapLink[] = [];
  for (let n = 0; n < squares; n += 1) links.push(link(String(n), String(n + 1), label));
  return links;
}

/** From where a jump starts to where it lands. */
export type Jumps = ReadonlyMap<number, number>;

/** One-way links for every jump, in order of where each starts. A square may start only one. */
export function jumpLinks(jumps: Jumps, label: string): MapLink[] {
  return [...jumps].sort(([a], [b]) => a - b).map(([from, to]) => link(String(from), String(to), label));
}

/** Ladders and snakes together, refusing a square that starts two. */
export function allJumps(...sets: Jumps[]): Jumps {
  const all = new Map<number, number>();
  for (const set of sets) {
    for (const [from, to] of set) {
      if (all.has(from)) throw new Error(`allJumps: square ${from} starts two jumps`);
      if (from === to) throw new Error(`allJumps: square ${from} jumps to itself`);
      all.set(from, to);
    }
  }
  return all;
}
