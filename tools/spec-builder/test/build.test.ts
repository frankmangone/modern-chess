import test from "node:test";
import assert from "node:assert/strict";
import { existsSync } from "node:fs";
import { build, toFile, type Game } from "../src/build.ts";
import { action, stage } from "../src/blocks.ts";
import { readSpec } from "../src/engine.ts";
import { bagDef, endCheck, mapBoard, mapPlace, piece, players, standardSections, turns, zone } from "../src/scaffold.ts";
import { stuck } from "../src/blocks.ts";

const engineBuilt = existsSync(new URL("../../../apps/docs/theme/pkg/modern_chess_wasm_bg.wasm", import.meta.url));
const needsEngine = { skip: engineBuilt ? false : "the engine is not built: run `make wasm`" };

/** The smallest game the engine reads: one place, one stage, a game that is over at once. */
const tiny: Game = {
  spec: {
    name: "TINY",
    board: mapBoard([mapPlace("a", [0, 0], 1)], []),
    players: players("A", "B"),
    turns: turns(["A", "B"], ["GO"]),
    pieces: [piece("P")],
    bags: [bagDef("BAG")],
    zones: [zone("Z", { A: ["a"], B: ["a"] })],
    actions: [action("GO", [stage("GO")])],
    endChecks: [endCheck(stuck(), "DRAW", { as: "NEXT" })],
  },
  page: {
    slug: "tiny",
    name: "Tiny",
    tagline: "t",
    description: "d",
    tags: ["x"],
    accent: "#000000",
    skin: { captured_panel: false },
    sections: standardSections("Rules."),
    featured: false,
    sortOrder: 1,
  },
  files: { spec: "tiny.json", page: "tiny-page.json" },
};

test("a spec's keys come in the order the committed ones have them", async () => {
  const { spec } = await build(tiny, { validate: false });
  assert.deepEqual(Object.keys(spec), ["format_version", "name", "board", "players", "turns", "pieces", "bags", "zones", "actions", "end_checks"]);
  assert.equal(spec.format_version, 1);
});

test("a spec leaves out the parts a game does not have, and a score comes last", async () => {
  const bare: Game = { ...tiny, spec: { ...tiny.spec, bags: undefined, zones: undefined, score: [{ term: "T", zone: "Z", where: {} }] } };
  const { spec } = await build(bare, { validate: false });
  assert.deepEqual(Object.keys(spec), ["format_version", "name", "board", "players", "turns", "pieces", "actions", "end_checks", "score"]);
});

test("a page's keys come in the order the committed ones have them", async () => {
  const { page } = await build(tiny, { validate: false });
  assert.deepEqual(Object.keys(page), ["slug", "name", "tagline", "description", "tags", "accent", "skin", "sections", "featured", "sort_order"]);
  assert.equal(page.sort_order, 1);
});

test("a file is two-space JSON with a final newline", () => {
  assert.equal(toFile({ a: [1, 2] }), '{\n  "a": [\n    1,\n    2\n  ]\n}\n');
});

test("the engine reads a whole game", needsEngine, async () => {
  await assert.doesNotReject(build(tiny));
});

test("a spec the engine refuses is refused with the engine's message", needsEngine, async () => {
  const wrong: Game = { ...tiny, spec: { ...tiny.spec, turns: turns(["A", "NOBODY"], ["GO"]) } };
  await assert.rejects(build(wrong), /the engine refuses TINY: .*NOBODY/s);
  await assert.rejects(readSpec("EMPTY", { format_version: 1 }), /the engine refuses EMPTY/);
});

test("validate: false does not ask the engine", async () => {
  const wrong: Game = { ...tiny, spec: { ...tiny.spec, turns: turns(["A", "NOBODY"], ["GO"]) } };
  await assert.doesNotReject(build(wrong, { validate: false }));
});
