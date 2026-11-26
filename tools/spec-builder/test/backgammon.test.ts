import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { backgammonArt } from "../src/art/backgammonArt.ts";
import { die, disc } from "../src/art/faces.ts";
import { allBornOff, backgammonZones, dieActions, dieLinks, pointAt, pointPlaces, rollTwoDice, type Side } from "../src/recipes/backgammon.ts";

const white: Side = { player: "WHITE", checker: "WHITE_CHECKER", label: "w", tray: "OFFW", bar: "White bar" };
const black: Side = { player: "BLACK", checker: "BLACK_CHECKER", label: "b", tray: "OFFB", bar: "Black bar" };
const saved = JSON.parse(readFileSync(new URL("../../../crates/engine/specs/backgammon.json", import.meta.url), "utf8"));

test("a point is drawn in its half: 1 to 6 bottom right, 7 to 12 bottom left, 13 to 18 top left, 19 to 24 top right", () => {
  assert.deepEqual(pointAt(1), [12, 0]);
  assert.deepEqual(pointAt(6), [7, 0]);
  assert.deepEqual(pointAt(7), [5, 0]);
  assert.deepEqual(pointAt(12), [0, 0]);
  assert.deepEqual(pointAt(13), [0, 7]);
  assert.deepEqual(pointAt(18), [5, 7]);
  assert.deepEqual(pointAt(19), [7, 7]);
  assert.deepEqual(pointAt(24), [12, 7]);
});

test("the points carry their starting checkers and the trays carry none", () => {
  const places = pointPlaces({ WHITE_CHECKER: { 24: 2 }, BLACK_CHECKER: { 1: 2, 24: 1 } }, [["OFFW", [13, 0]]], 15);
  assert.equal(places.length, 25);
  assert.deepEqual(places[23]?.start, { WHITE_CHECKER: 2, BLACK_CHECKER: 1 });
  assert.deepEqual(places[0]?.start, { BLACK_CHECKER: 2 });
  assert.equal(places[1]?.start, undefined);
  assert.deepEqual(places[24], { name: "OFFW", at: [13, 0], capacity: 15 });
});

test("every die number links each point to the point that many away, each side its own way", () => {
  const links = dieLinks("w", "b");
  // Point 1 has no point below it, so its first link goes up; point 2 then has both.
  assert.deepEqual(links.slice(0, 3), [{ from: "1", to: "2", label: "b1" }, { from: "2", to: "1", label: "w1" }, { from: "2", to: "3", label: "b1" }]);
  // For a die n: (24 - n) links one way and (24 - n) the other.
  assert.equal(links.length, [1, 2, 3, 4, 5, 6].reduce((sum, n) => sum + 2 * (24 - n), 0));
  assert.equal(links.some((each) => "from" in each && each.from === "1" && each.label === "w1"), false);
});

test("the zones are the committed ones", () => {
  assert.deepEqual(backgammonZones(white, black), saved.zones);
});

test("the roll is the committed one: two dice, doubles counted twice, then the stage of play", () => {
  assert.deepEqual(rollTwoDice(), saved.actions[0]);
});

test("every die can be spent on a move, an entry from the bar and a bearing off, as committed", () => {
  const actions = dieActions(white, black);
  assert.equal(actions.length, 18);
  assert.deepEqual(actions.map((each) => each.name).slice(0, 7), ["MOVE_1", "MOVE_2", "MOVE_3", "MOVE_4", "MOVE_5", "MOVE_6", "ENTER_1"]);
  assert.ok(actions.every((each) => each.stage === "PLAY"));
  assert.deepEqual(actions, saved.actions.slice(1));
});

test("the game is won when a tray holds all the checkers", () => {
  assert.deepEqual(allBornOff(15), saved.end_checks[0].when);
});

test("a die has the pips of its number and no other face exists", () => {
  assert.equal(die(1).match(/<circle/g)?.length, 1);
  assert.equal(die(6).match(/<circle/g)?.length, 6);
  assert.throws(() => die(7), /die: there is no face 7/);
  assert.throws(() => die(0), /die: there is no face 0/);
  assert.match(disc("#fff", "#000", "#eee"), /fill="#fff" stroke="#000"/);
});

test("the table's art has twenty-four triangles, each numbered, in a 15 by 9 box", () => {
  const art = backgammonArt();
  assert.match(art, /viewBox="0 0 15 9"/);
  assert.equal(art.match(/<polygon/g)?.length, 24);
  for (let p = 1; p <= 24; p += 1) assert.ok(art.includes(`>${p}</text>`), `point ${p}`);
});
