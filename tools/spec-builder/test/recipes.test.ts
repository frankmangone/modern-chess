import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { atBag, codes, countOf } from "../src/blocks.ts";
import { codeFaces, countFaces, throwIntoBag } from "../src/recipes/dice.ts";
import { firstInZone, walkAndJump } from "../src/recipes/race.ts";
import { allJumps, jumpLinks, squareAt, stepLinks, trackPlaces } from "../src/recipes/track.ts";
import { bag } from "../src/blocks.ts";

test("squares run left to right on the first row, then right to left, and so on", () => {
  assert.deepEqual(squareAt(1, 10), [0, 0]);
  assert.deepEqual(squareAt(10, 10), [9, 0]);
  assert.deepEqual(squareAt(11, 10), [9, 1]);
  assert.deepEqual(squareAt(20, 10), [0, 1]);
  assert.deepEqual(squareAt(21, 10), [0, 2]);
  assert.deepEqual(squareAt(100, 10), [0, 9]);
  assert.deepEqual(squareAt(5, 4), [3, 1]);
});

test("squares count from one", () => {
  assert.throws(() => squareAt(0, 10), /squares count from 1/);
  assert.throws(() => squareAt(1.5, 10), /squares count from 1/);
});

test("a track has a pad named 0 and every square, with room for the pieces that stand on them", () => {
  const places = trackPlaces({ squares: 4, rowLength: 2, capacity: 3, pad: { at: [-1, 0], start: { A: 1 } } });
  assert.deepEqual(places.map((place) => place.name), ["0", "1", "2", "3", "4"]);
  assert.deepEqual(places[0], { name: "0", at: [-1, 0], capacity: 3, start: { A: 1 } });
  assert.deepEqual(places[3], { name: "3", at: [1, 1], capacity: 3 });
});

test("steps link each square to the next from the pad to the last, one way", () => {
  assert.deepEqual(stepLinks(3, "step"), [
    { from: "0", to: "1", label: "step" },
    { from: "1", to: "2", label: "step" },
    { from: "2", to: "3", label: "step" },
  ]);
});

test("jump links come in order of where they start, whatever order they were written in", () => {
  const jumps = new Map([[9, 31], [1, 38], [16, 6]]);
  assert.deepEqual(jumpLinks(jumps, "jump").map((each) => ("from" in each ? each.from : "")), ["1", "9", "16"]);
});

test("a square may start one jump, and no jump lands where it starts", () => {
  assert.throws(() => allJumps(new Map([[4, 14]]), new Map([[4, 2]])), /square 4 starts two jumps/);
  assert.throws(() => allJumps(new Map([[4, 4]])), /square 4 jumps to itself/);
  assert.deepEqual([...allJumps(new Map([[1, 2]]), new Map([[3, 1]]))], [[1, 2], [3, 1]]);
});

test("a die that adds as many pieces as it shows, and one that adds a piece for each face", () => {
  assert.deepEqual(countFaces("PIP", 3), [{ PIP: 1 }, { PIP: 2 }, { PIP: 3 }]);
  assert.deepEqual(codeFaces("D", 3), [{ D1: 1 }, { D2: 1 }, { D3: 1 }]);
  assert.equal(countFaces("PIP").length, 6);
});

test("a throw empties the bag first, then throws into it", () => {
  const steps = throwIntoBag(bag("TRAY"), countFaces("PIP", 2));
  assert.equal(JSON.stringify(steps), '[{"effect":"REMOVE","at":{"bag":"TRAY"},"count":"ALL"},{"effect":"ROLL","at":{"bag":"TRAY"},"faces":[{"PIP":1},{"PIP":2}]}]');
  assert.match(JSON.stringify(throwIntoBag(bag("DICE", "MOVER"), codeFaces("D", 2), 2)), /"dice":2/);
});

/** The committed Snakes and Ladders spec, to compare what a recipe writes with what is saved. */
const saved = JSON.parse(readFileSync(new URL("../../../crates/engine/specs/snakes_and_ladders.json", import.meta.url), "utf8"));

test("walkAndJump writes the turn the committed Snakes and Ladders spec has for each side", () => {
  const pips = countOf(atBag("TRAY"), codes("PIP"));
  const [, , red, blue] = saved.actions[0].do;
  assert.deepEqual(walkAndJump({ player: "RED", token: "RED_TOKEN", steps: pips, step: "step", jump: "jump" }), red);
  assert.deepEqual(walkAndJump({ player: "BLUE", token: "BLUE_TOKEN", steps: pips, step: "step", jump: "jump" }), blue);
});

test("firstInZone is the committed end check's test", () => {
  assert.deepEqual(firstInZone("FINISH"), saved.end_checks[0].when);
});
