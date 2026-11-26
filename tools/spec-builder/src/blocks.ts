// Typed constructors for the blocks a spec is written in. The types follow specs/SPEC_FORMAT.md;
// the Rust spec types stay the authority, and `build` asks the engine to read what is built. A
// helper writes its keys in the order the committed specs have them, so what is built can be
// compared byte for byte with what is saved, and refuses a block that cannot be right when it is
// written, naming the helper, not when the engine reads the spec.

/** A name the engine reads: a place, a bag, a zone, a stage, a piece code. */
function name(what: string, value: string): string {
  if (typeof value !== "string" || value.length === 0) throw new Error(`${what}: a name is needed`);
  return value;
}

/** A count of pieces: a whole number from 1 up, or "ALL". */
export type Count = number | "ALL";

function count1(what: string, value: Count): Count {
  if (value === "ALL") return value;
  if (!Number.isInteger(value) || value < 1) throw new Error(`${what}: the count must be 1 or more, or "ALL" (got ${value})`);
  return value;
}

// ---------- Places and bags ----------

/** A place of the board: by name, or `TARGET` (the one the player chose), `EACH` (the one a loop
 *  is at) or `LAST` (where the last move of a loop ended). */
export interface PlaceRef {
  place: string;
}
/** A bag: `of` says whose (`MOVER`, `ENEMY`), left out for a bag nobody owns. */
export interface BagRef {
  bag: string;
  of?: string;
}
export type At = PlaceRef | BagRef;

export const TARGET = "TARGET";
export const EACH = "EACH";
export const LAST = "LAST";

export function place(at: string): PlaceRef {
  return { place: name("place", at) };
}

export function bag(bagName: string, of?: string): BagRef {
  const ref: BagRef = { bag: name("bag", bagName) };
  if (of !== undefined) ref.of = name("bag owner", of);
  return ref;
}

// ---------- Filters ----------

/** Which pieces: by code, by owner (`ALLY`, `ENEMY`), or both. */
export interface Where {
  codes?: string[];
  owner?: string;
}

export function codes(...list: string[]): Where {
  if (list.length === 0) throw new Error("codes: at least one piece code is needed");
  return { codes: list.map((code) => name("piece code", code)) };
}

// ---------- Regions ----------

export type Region =
  | { region: "BOARD"; where?: Where }
  | { region: "AT"; at: At }
  | { region: "AROUND"; of: PlaceRef; through: string }
  | { region: "FOLLOW"; from: PlaceRef; steps: Steps; through?: string; skipping?: string };

/** How many steps a walk takes: the number of pieces a filter finds in a region. */
export interface Steps {
  count: { in: Region; where?: Where };
}

/** The number of pieces of a kind in a region, as the steps of a walk. */
export function countOf(region: Region, where?: Where): Steps {
  return where === undefined ? { count: { in: region } } : { count: { in: region, where } };
}

export function onBoard(where?: Where): Region {
  return where === undefined ? { region: "BOARD" } : { region: "BOARD", where };
}

export function atBag(bagName: string, of?: string): Region {
  return { region: "AT", at: bag(bagName, of) };
}

export function atPlace(at: string): Region {
  return { region: "AT", at: place(at) };
}

/** The places one link away through a label. */
export function around(of: string, through: string): Region {
  return { region: "AROUND", of: place(of), through: name("link label", through) };
}

/** The places a walk of `steps` passes through from a place, along a label (or any), skipping a zone. */
export function follow(from: string, steps: Steps, options: { through?: string; skipping?: string } = {}): Region {
  const region: Region = { region: "FOLLOW", from: place(from), steps };
  if (options.through !== undefined) region.through = name("link label", options.through);
  if (options.skipping !== undefined) region.skipping = name("zone", options.skipping);
  return region;
}

// ---------- Tests ----------

export type Test =
  | { test: "COUNT"; in: Region; where?: Where; min?: number; max?: number }
  | { test: "IN_ZONE"; at: PlaceRef; zone: string }
  | { test: "PLAYER"; is: string[] }
  | { test: "SOME"; in: Region; that: Test }
  | { test: "ALL" | "ANY"; of: Test[] }
  | { test: "NOT"; of: Test }
  | { test: "STUCK" };

/** That a region holds a number of pieces, optionally of a kind, within bounds. */
export function count(region: Region, where?: Where, bounds: { min?: number; max?: number } = {}): Test & { test: "COUNT" } {
  const { min, max } = bounds;
  for (const [what, value] of [["min", min], ["max", max]] as const) {
    if (value !== undefined && (!Number.isInteger(value) || value < 0)) throw new Error(`count: ${what} must be 0 or more (got ${value})`);
  }
  if (min !== undefined && max !== undefined && min > max) throw new Error(`count: min ${min} is above max ${max}`);
  const test: Test & { test: "COUNT" } = { test: "COUNT", in: region };
  if (where !== undefined) test.where = where;
  if (min !== undefined) test.min = min;
  if (max !== undefined) test.max = max;
  return test;
}

export function inZone(at: string, zone: string): Test {
  return { test: "IN_ZONE", at: place(at), zone: name("zone", zone) };
}

export function playerIs(...names: string[]): Test {
  if (names.length === 0) throw new Error("playerIs: at least one player is needed");
  return { test: "PLAYER", is: names.map((player) => name("player", player)) };
}

/** True when some place of the region passes the test, which sees that place as `EACH`. */
export function some(region: Region, that: Test): Test {
  return { test: "SOME", in: region, that };
}

function several(helper: "all" | "any", tests: Test[]): Test {
  if (tests.length === 0) throw new Error(`${helper}: at least one test is needed`);
  return { test: helper === "all" ? "ALL" : "ANY", of: tests };
}

export function all(...tests: Test[]): Test {
  return several("all", tests);
}

export function any(...tests: Test[]): Test {
  return several("any", tests);
}

export function not(test: Test): Test {
  return { test: "NOT", of: test };
}

export function stuck(): Test {
  return { test: "STUCK" };
}

// ---------- Effects, choices and conditions ----------

export type Effect =
  | { effect: "MOVE"; from: At; to: At; what: Where; count: Count }
  | { effect: "REMOVE"; at: At; what?: Where; count: Count }
  | { effect: "PLACE"; at: At; code: string; count: Count }
  | { effect: "ROLL"; at: BagRef; dice?: number; faces: Record<string, number>[] }
  | { effect: "FOR_EACH"; in: Region; do: Step[] }
  | { effect: "STAGE"; name: string };

export type Choice = { choose: "PLACE"; in: Region };
export type If = { if: Test; then: Step[]; else?: Step[] };
/** One thing an action does, in order: a choice, a test that must hold, an effect or a condition. */
export type Step = Choice | Test | Effect | If;

export function choosePlace(region: Region): Choice {
  return { choose: "PLACE", in: region };
}

export function move(from: At, to: At, what: Where, pieces: Count): Effect {
  return { effect: "MOVE", from, to, what, count: count1("move", pieces) };
}

export function remove(at: At, pieces: Count, what?: Where): Effect {
  const effect: Effect = { effect: "REMOVE", at, count: count1("remove", pieces) };
  return what === undefined ? effect : { effect: "REMOVE", at, what, count: effect.count };
}

export function putDown(at: At, code: string, pieces: Count): Effect {
  return { effect: "PLACE", at, code: name("piece code", code), count: count1("place", pieces) };
}

/** Throws dice into a bag: each face is what one throw adds to it. */
export function roll(at: BagRef, faces: Record<string, number>[], dice?: number): Effect {
  if (faces.length === 0) throw new Error("roll: at least one face is needed");
  if (dice !== undefined && (!Number.isInteger(dice) || dice < 1)) throw new Error(`roll: dice must be 1 or more (got ${dice})`);
  return dice === undefined ? { effect: "ROLL", at, faces } : { effect: "ROLL", at, dice, faces };
}

export function forEach(region: Region, ...steps: Step[]): Effect {
  if (steps.length === 0) throw new Error("forEach: it needs something to do");
  return { effect: "FOR_EACH", in: region, do: steps };
}

export function stage(stageName: string): Effect {
  return { effect: "STAGE", name: name("stage", stageName) };
}

/** `then` when the test holds, else `otherwise` when given. */
export function when(test: Test, then: Step[], otherwise?: Step[]): If {
  if (then.length === 0) throw new Error("when: it needs something to do");
  return otherwise === undefined ? { if: test, then } : { if: test, then, else: otherwise };
}

// ---------- Actions ----------

export interface Action {
  name: string;
  stage?: string;
  do: Step[];
}

/** A named action a player may take, in a stage of the turn when given. */
export function action(actionName: string, steps: Step[], inStage?: string): Action {
  if (steps.length === 0) throw new Error(`action ${actionName}: it needs something to do`);
  return inStage === undefined ? { name: name("action", actionName), do: steps } : { name: name("action", actionName), stage: name("stage", inStage), do: steps };
}
