// The rules of a race with hitting, played with two dice kept in each player's own bag: a die
// is spent to move one checker that many points, to enter one from the bar, or to bear one off.
// Points are numbered as White sees them: White moves from 24 down to 1 and bears off past 1,
// Black the other way. Each die number has its own link label per side (`w3`, `b3`) so a move
// of that many points is a single step.
import {
  EACH, TARGET, action, all, any, around, atBag, atPlace, bag, choosePlace, codes, count, forEach, inZone, move, not, onBoard, place,
  playerIs, putDown, remove, roll, some, stage, when,
  type Action, type At, type BagRef, type Step, type Test,
} from "../blocks.ts";
import { codeFaces } from "./dice.ts";
import { link, mapPlace, zone, type MapLink, type MapPlace, type ZoneDef } from "../scaffold.ts";

export interface Side {
  player: string;
  /** The checker's piece code. */
  checker: string;
  /** Prefix of the link labels this side moves along: `w` gives `w1` to `w6`. */
  label: string;
  /** The tray this side bears off into, a place. */
  tray: string;
  /** The bag this side's captured checkers wait in. */
  bar: string;
}

export const POINTS = 24;
export const DICE_BAG = "DICE";
/** The stage in which dice are spent, after the roll. */
export const PLAY = "PLAY";

/** Where point `p` is drawn: [column, row]. The left half is columns 0 to 5, the bar column 6,
 *  the right half columns 7 to 12; the top row is 7 and the bottom row 0. */
export function pointAt(p: number): [number, number] {
  if (p >= 13) return [p <= 18 ? p - 13 : p - 19 + 7, 7];
  return [p >= 7 ? 12 - p : 6 - p + 7, 0];
}

/** The 24 points, each with the checkers that start on it, then the two trays. */
export function pointPlaces(start: Record<string, Record<number, number>>, trays: [string, [number, number]][], capacity: number): MapPlace[] {
  const places: MapPlace[] = [];
  for (let p = 1; p <= POINTS; p += 1) {
    const here: Record<string, number> = {};
    for (const [code, points] of Object.entries(start)) {
      const n = points[p];
      if (n !== undefined) here[code] = n;
    }
    places.push(mapPlace(String(p), pointAt(p), capacity, here));
  }
  for (const [name, at] of trays) places.push(mapPlace(name, at, capacity));
  return places;
}

/** For every die number, a link from each point to the point that many away, in each direction
 *  (`w` towards 1, `b` towards 24), when there is one. */
export function dieLinks(white: string, black: string): MapLink[] {
  const links: MapLink[] = [];
  for (let n = 1; n <= 6; n += 1) {
    for (let p = 1; p <= POINTS; p += 1) {
      if (p - n >= 1) links.push(link(String(p), String(p - n), `${white}${n}`));
      if (p + n <= POINTS) links.push(link(String(p), String(p + n), `${black}${n}`));
    }
  }
  return links;
}

const names = (points: Iterable<number>): string[] => [...points].map(String);
const range = (from: number, to: number): number[] => Array.from({ length: Math.max(0, to - from) }, (_, i) => from + i);

/** The zones the rules ask about, for White (home 1-6) and Black (home 19-24): `HOME`, `TRAY`,
 *  `SAFE` (home or tray), `P1`-`P6` (the point of that number from each side's end), `FARTHER1`-
 *  `FARTHER5` (every point farther from the end than that), `ENTRY1`-`ENTRY6` (where a die enters). */
export function backgammonZones(white: Side, black: Side): ZoneDef[] {
  const both = (w: Iterable<number>, b: Iterable<number>, extra?: [string, string]): Record<string, string[]> => ({
    [white.player]: [...(extra ? [extra[0]] : []), ...names(w)],
    [black.player]: [...(extra ? [extra[1]] : []), ...names(b)],
  });
  const zones: ZoneDef[] = [
    zone("HOME", both(range(1, 7), range(19, 25))),
    zone("TRAY", { [white.player]: [white.tray], [black.player]: [black.tray] }),
    zone("SAFE", both(range(1, 7), range(19, 25), [white.tray, black.tray])),
  ];
  for (let k = 1; k <= 6; k += 1) zones.push(zone(`P${k}`, both([k], [25 - k])));
  for (let k = 1; k <= 5; k += 1) zones.push(zone(`FARTHER${k}`, both(range(k + 1, 25), range(1, 25 - k))));
  for (let n = 1; n <= 6; n += 1) zones.push(zone(`ENTRY${n}`, both([25 - n], [n])));
  return zones;
}

// ---------- The blocks of a move ----------

const mover = (name: string): BagRef => bag(name, "MOVER");
const hasDie = (n: number): Test => count(atBag(DICE_BAG, "MOVER"), codes(`D${n}`), { min: 1 });
const spend = (n: number): Step => remove(mover(DICE_BAG), 1, codes(`D${n}`));
const stay: Step = stage(PLAY);
const barOf = (side: Side): At => bag(side.bar);

/** The player has nothing on the bar. */
const noBar = (side: Side): Test => count({ region: "AT", at: barOf(side) }, undefined, { min: 0, max: 0 });

/** A lone enemy checker on the target point goes to its owner's bar. */
function hit(side: Side, other: Side): Step {
  return when(count(atPlace(TARGET), codes(other.checker), { min: 1 }), [move(place(TARGET), barOf(other), codes(other.checker), 1)]);
}

/** The target point holds no more than one enemy checker. */
const openPoint = (other: Side): Test => count(atPlace(TARGET), codes(other.checker), { min: 0, max: 1 });

function moveWith(side: Side, other: Side, n: number): Step[] {
  const behind = around(TARGET, `${other.label}${n}`);
  return [
    hasDie(n),
    noBar(side),
    choosePlace(onBoard()),
    some(behind, count(atPlace(EACH), codes(side.checker), { min: 1 })),
    openPoint(other),
    hit(side, other),
    forEach(behind, move(place(EACH), place(TARGET), codes(side.checker), 1)),
    spend(n),
    stay,
  ];
}

function enterWith(side: Side, other: Side, n: number): Step[] {
  return [
    hasDie(n),
    choosePlace(onBoard()),
    inZone(TARGET, `ENTRY${n}`),
    openPoint(other),
    hit(side, other),
    move(barOf(side), place(TARGET), codes(side.checker), 1),
    spend(n),
    stay,
  ];
}

function bearOffWith(side: Side, n: number): Step[] {
  const mine = count(atPlace(EACH), codes(side.checker), { min: 1 });
  const outside = some(onBoard(), all(not(inZone(EACH, "SAFE")), mine));
  // A checker may go off with this die when it stands on the point of that number, or on a lower
  // one with nothing farther back.
  const ways: Test[] = [inZone(EACH, `P${n}`)];
  for (let k = 1; k < n; k += 1) {
    const farther = some(onBoard(), all(inZone(EACH, `FARTHER${k}`), mine));
    ways.push(all(inZone(EACH, `P${k}`), not(farther)));
  }
  const goes = all(mine, any(...ways));
  return [
    hasDie(n),
    noBar(side),
    not(outside),
    choosePlace(onBoard()),
    inZone(TARGET, "TRAY"),
    some(onBoard(), goes),
    forEach(onBoard(), when(goes, [move(place(EACH), place(TARGET), codes(side.checker), 1)])),
    spend(n),
    stay,
  ];
}

/** One action of a die number that does one thing for each side. */
function perSide(sides: [Side, Side], build: (side: Side, other: Side, n: number) => Step[], n: number): Step[] {
  const [first, second] = sides;
  return [when(playerIs(first.player), build(first, second, n), build(second, first, n))];
}

/** The roll: two dice into the player's bag, and a double counted twice over, so that it can be
 *  spent four times. Then the turn moves to the stage where dice are spent. */
export function rollTwoDice(): Action {
  const doubles = Array.from({ length: 6 }, (_, i) => i + 1).map((k) =>
    when(count(atBag(DICE_BAG, "MOVER"), codes(`D${k}`), { min: 2 }), [putDown(mover(DICE_BAG), `D${k}`, 2)]),
  );
  return action("ROLL", [remove(mover(DICE_BAG), "ALL"), remove(bag(DICE_BAG, "ENEMY"), "ALL"), roll(mover(DICE_BAG), codeFaces("D"), 2), ...doubles, stay]);
}

/** Every action a die can be spent on: moving a checker (`MOVE_n`), entering one from the bar
 *  (`ENTER_n`) and bearing one off (`OFF_n`), for n from 1 to 6. */
export function dieActions(white: Side, black: Side): Action[] {
  const sides: [Side, Side] = [white, black];
  const kinds: [string, (side: Side, other: Side, n: number) => Step[]][] = [
    ["MOVE", moveWith],
    ["ENTER", enterWith],
    ["OFF", (side, _other, n) => bearOffWith(side, n)],
  ];
  const actions: Action[] = [];
  for (const [kind, build] of kinds) {
    for (let n = 1; n <= 6; n += 1) actions.push(action(`${kind}_${n}`, perSide(sides, build, n), PLAY));
  }
  return actions;
}

/** Some tray holds all fifteen checkers: the game is won. */
export function allBornOff(checkers: number): Test {
  return some(onBoard(), all(inZone(EACH, "TRAY"), count(atPlace(EACH), undefined, { min: checkers })));
}
