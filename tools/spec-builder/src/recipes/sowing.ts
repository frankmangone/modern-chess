// Sowing games (the Mancala family, here Kalah): a ring of pits with a store at each end, stones
// picked up from one pit and dropped one at a time into every place that follows, and a capture
// across the board. Nothing here is special to the engine: a pit, a store and a stone are a
// place, a place and a piece, and the ring is a chain of links.
import {
  EACH, LAST, TARGET, action, all, around, atPlace, choosePlace, codes, count, countOf, follow, forEach, inZone, move, not, onBoard,
  place, some, stage, stuck, when, type Action,
} from "../blocks.ts";
import { between, endCheck, link, mapPlace, scoreTerm, zone, type EndCheck, type MapLink, type MapPlace, type ScoreTerm, type ZoneDef } from "../scaffold.ts";

/** One player's side of the board. */
export interface Row {
  player: string;
  /** The pits are named this letter and their number from 1: `a1` to `a6`. */
  prefix: string;
  /** The store pits drain into, where the player's stones are kept. */
  store: string;
}

export interface Ring {
  pits: number;
  /** The stones in every pit as the game begins. */
  seeds: number;
  stone: string;
  south: Row;
  north: Row;
}

const pitNames = (row: Row, pits: number): string[] => Array.from({ length: pits }, (_, i) => `${row.prefix}${i + 1}`);

/** The pits and the stores. The south row runs left to right along the bottom, the north row back
 *  along the top, and the stores stand at the two ends, between the rows. Room for every stone. */
export function ringPlaces(ring: Ring): MapPlace[] {
  const { pits, seeds, stone, south, north } = ring;
  const room = 2 * pits * seeds;
  const start = { [stone]: seeds };
  return [
    ...pitNames(south, pits).map((name, i) => mapPlace(name, [i + 1, 0], room, start)),
    mapPlace(south.store, [pits + 1, 1], room),
    ...pitNames(north, pits).map((name, i) => mapPlace(name, [pits - i, 2], room, start)),
    mapPlace(north.store, [0, 1], room),
  ];
}

/** The ring itself, one way round (south pits, south store, north pits, north store, and back);
 *  `across`, joining each pit to the one opposite; and `store`, from every pit to its own store. */
export function ringLinks(ring: Ring): MapLink[] {
  const { pits, south, north } = ring;
  const southPits = pitNames(south, pits);
  const northPits = pitNames(north, pits);
  const round = [...southPits, south.store, ...northPits, north.store];
  const links: MapLink[] = round.map((name, i) => link(name, round[(i + 1) % round.length] as string));
  southPits.forEach((name, i) => links.push(between(name, northPits[pits - 1 - i] as string, "across")));
  for (const row of [south, north]) for (const name of pitNames(row, pits)) links.push(link(name, row.store, "store"));
  return links;
}

/** `PITS` (a player's pits), `OTHER_STORE` (the store a player's sowing skips), `MINE` (pits and
 *  store: what a player keeps) and `STORE`. */
export function ringZones(ring: Ring): ZoneDef[] {
  const { pits, south, north } = ring;
  const own = (row: Row) => [...pitNames(row, pits), row.store];
  return [
    zone("PITS", { [south.player]: pitNames(south, pits), [north.player]: pitNames(north, pits) }),
    zone("OTHER_STORE", { [south.player]: [north.store], [north.player]: [south.store] }),
    zone("MINE", { [south.player]: own(south), [north.player]: own(north) }),
    zone("STORE", { [south.player]: [south.store], [north.player]: [north.store] }),
  ];
}

/** The stage a turn goes to when its last stone lands in the player's own store: another turn. */
export const AGAIN = "AGAIN";

/** Sowing: choose a pit that holds stones and drop them one at a time into each place that
 *  follows, skipping the opponent's store. A last stone in your own store is another turn. A last
 *  stone in an empty pit of your own, with stones in the pit across, takes them all, and that
 *  last stone, into your store. */
export function sow(ring: Ring): Action {
  const mine = codes(ring.stone);
  const sowing = follow(TARGET, countOf(atPlace(TARGET), mine), { skipping: "OTHER_STORE" });
  const captures = all(inZone(LAST, "PITS"), count(atPlace(LAST), mine, { max: 1 }), count(around(LAST, "across"), mine));
  return action("SOW", [
    choosePlace(onBoard()),
    inZone(TARGET, "PITS"),
    count(atPlace(TARGET)),
    forEach(sowing, move(place(TARGET), place(EACH), mine, 1)),
    when(inZone(LAST, "STORE"), [stage(AGAIN)]),
    when(captures, [
      forEach(around(LAST, "across"), move(place(EACH), place(LAST), mine, "ALL")),
      forEach(around(LAST, "store"), move(place(LAST), place(EACH), mine, "ALL")),
    ]),
  ]);
}

/** The game ends when no pit holds a stone, or when the player to move has no move; then whoever
 *  kept the most stones wins. */
export function ringEnds(ring: Ring): EndCheck[] {
  return [
    endCheck(not(some(onBoard(), all(inZone(EACH, "PITS"), count(atPlace(EACH), codes(ring.stone))))), "HIGHEST"),
    endCheck(stuck(), "HIGHEST", { as: "NEXT" }),
  ];
}

/** What a player keeps: the stones in their pits and their store. */
export function ringScore(ring: Ring): ScoreTerm[] {
  return [scoreTerm("KEPT", "MINE", codes(ring.stone))];
}
