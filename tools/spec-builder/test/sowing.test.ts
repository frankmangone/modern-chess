import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { ringEnds, ringLinks, ringPlaces, ringScore, ringZones, sow, type Ring } from "../src/recipes/sowing.ts";

const kalah: Ring = {
  pits: 6,
  seeds: 4,
  stone: "STONE",
  south: { player: "SOUTH", prefix: "a", store: "sa" },
  north: { player: "NORTH", prefix: "b", store: "sb" },
};
const saved = JSON.parse(readFileSync(new URL("../../../crates/engine/specs/kalah.json", import.meta.url), "utf8"));

test("the ring has a row of pits along the bottom, a store at each end, and a row back along the top", () => {
  const places = ringPlaces(kalah);
  assert.equal(places.length, 14);
  assert.deepEqual(places.find((p) => p.name === "a1"), { name: "a1", at: [1, 0], capacity: 48, start: { STONE: 4 } });
  assert.deepEqual(places.find((p) => p.name === "sa"), { name: "sa", at: [7, 1], capacity: 48 });
  assert.deepEqual(places.find((p) => p.name === "b1")?.at, [6, 2]);
  assert.deepEqual(places.find((p) => p.name === "sb")?.at, [0, 1]);
});

test("there is room in every place for every stone", () => {
  const small = ringPlaces({ ...kalah, pits: 3, seeds: 2 });
  assert.ok(small.every((p) => p.capacity === 12));
});

test("the ring goes one way round, each pit faces the one opposite, and every pit drains into its store", () => {
  const links = ringLinks(kalah);
  assert.equal(links.length, 14 + 6 + 12);
  assert.deepEqual(links.slice(0, 2), [{ from: "a1", to: "a2" }, { from: "a2", to: "a3" }]);
  assert.deepEqual(links[13], { from: "sb", to: "a1" });
  assert.deepEqual(links[14], { between: ["a1", "b6"], label: "across" });
  assert.deepEqual(links[20], { from: "a1", to: "sa", label: "store" });
});

test("a smaller ring is the same game with fewer pits", () => {
  const links = ringLinks({ ...kalah, pits: 2 });
  assert.deepEqual(links.filter((l) => "between" in l), [{ between: ["a1", "b2"], label: "across" }, { between: ["a2", "b1"], label: "across" }]);
});

test("the zones, the sowing, the ends and the score are the committed ones", () => {
  assert.deepEqual(ringZones(kalah), saved.zones);
  assert.deepEqual(sow(kalah), saved.actions[0]);
  assert.deepEqual(ringEnds(kalah), saved.end_checks);
  assert.deepEqual(ringScore(kalah), saved.score);
});
