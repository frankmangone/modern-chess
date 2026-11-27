import test from "node:test";
import assert from "node:assert/strict";
import { stuck } from "../src/blocks.ts";
import { bagDef, between, endCheck, link, mapBoard, mapPlace, piece, players, scoreTerm, standardSections, turns, zone } from "../src/scaffold.ts";

const text = (value: unknown): string => JSON.stringify(value);

test("a place lists its start only when something starts there", () => {
  assert.equal(text(mapPlace("a1", [1, 0], 48, { STONE: 4 })), '{"name":"a1","at":[1,0],"capacity":48,"start":{"STONE":4}}');
  assert.equal(text(mapPlace("sa", [7, 1], 48)), '{"name":"sa","at":[7,1],"capacity":48}');
  assert.equal(text(mapPlace("sa", [7, 1], 48, {})), '{"name":"sa","at":[7,1],"capacity":48}');
});

test("a place refuses a missing name and a capacity under one", () => {
  assert.throws(() => mapPlace("", [0, 0], 1), /mapPlace: a name is needed/);
  assert.throws(() => mapPlace("a", [0, 0], 0), /mapPlace a: the capacity must be 1 or more/);
});

test("links are one-way, labelled one-way, or labelled two-way", () => {
  assert.equal(text(link("a", "b")), '{"from":"a","to":"b"}');
  assert.equal(text(link("a", "b", "store")), '{"from":"a","to":"b","label":"store"}');
  assert.equal(text(between("a", "b", "across")), '{"between":["a","b"],"label":"across"}');
  assert.throws(() => between("a", "b", ""), /between: a label is needed/);
});

test("a map refuses two places of one name and a link to a place it does not have", () => {
  const a = mapPlace("a", [0, 0], 1);
  assert.throws(() => mapBoard([a, a], []), /two places are named a/);
  assert.throws(() => mapBoard([a], [link("a", "z")]), /names z, which is not a place/);
  assert.throws(() => mapBoard([a], [between("z", "a", "x")]), /names z, which is not a place/);
  assert.equal(text(mapBoard([a], [])), '{"shape":"map","places":[{"name":"a","at":[0,0],"capacity":1}],"links":[]}');
});

test("players list no pieces of their own, turns list stages only when there are some", () => {
  assert.equal(text(players("RED", "BLUE")), '[{"name":"RED","starting_positions":[]},{"name":"BLUE","starting_positions":[]}]');
  assert.equal(text(turns(["RED", "BLUE"])), '{"order":["RED","BLUE"]}');
  assert.equal(text(turns(["WHITE", "BLACK"], ["PLAY"])), '{"order":["WHITE","BLACK"],"stages":[{"name":"PLAY"}]}');
  assert.throws(() => players(), /players: at least one player/);
  assert.throws(() => turns([]), /turns: the order needs a player/);
});

test("a piece's name is its code in lower case unless given", () => {
  assert.equal(text(piece("RED_TOKEN")), '{"code":"RED_TOKEN","name":"red token","moves":[]}');
  assert.equal(text(piece("STONE", "seed")), '{"code":"STONE","name":"seed","moves":[]}');
  assert.throws(() => piece(""), /piece: a code is needed/);
});

test("bags and zones", () => {
  assert.equal(text(bagDef("TRAY")), '{"name":"TRAY"}');
  assert.equal(text(bagDef("DICE", { eachPlayer: true, capacity: 4 })), '{"name":"DICE","for":"EACH_PLAYER","capacity":4}');
  assert.equal(text(zone("FINISH", { RED: ["100"] })), '{"name":"FINISH","places":{"RED":["100"]}}');
  assert.throws(() => bagDef(""), /bagDef: a name is needed/);
  assert.throws(() => zone("", {}), /zone: a name is needed/);
});

test("an end check writes `as` first when it has one", () => {
  assert.equal(text(endCheck(stuck(), "HIGHEST", { as: "NEXT" })), '{"as":"NEXT","when":{"test":"STUCK"},"then":{"outcome":"HIGHEST"}}');
  assert.equal(text(endCheck(stuck(), "WIN")), '{"when":{"test":"STUCK"},"then":{"outcome":"WIN"}}');
  assert.equal(text(scoreTerm("KEPT", "MINE", { codes: ["STONE"] })), '{"term":"KEPT","zone":"MINE","where":{"codes":["STONE"]}}');
});

test("a page has its rules, then the board to try, then the lobbies", () => {
  assert.deepEqual(standardSections("Rules."), [{ type: "how_to_play", markdown: "Rules." }, { type: "preview" }, { type: "open_lobbies" }]);
  assert.throws(() => standardSections("  "), /standardSections: the rules need words/);
});

test("a stone that slides moves along one link to an empty place", async () => {
  const { slider, bothWays, playersAt } = await import("../src/scaffold.ts");
  assert.equal(text(slider("STONE")), '{"code":"STONE","name":"stone","moves":[{"id":0,"step":"next","do":[{"test":"NOT","of":{"test":"COUNT","in":{"region":"AT","at":{"place":"TARGET"}}}},{"effect":"MOVE","from":{"place":"START"},"to":{"place":"TARGET"}}]}]}');
  assert.deepEqual(bothWays("a", "b"), ["a", "b"]);
  assert.throws(() => bothWays("", "b"), /bothWays: both ends need a name/);
  assert.equal(text(playersAt("STONE", { WHITE: ["a", "b"], BLACK: [] })), '[{"name":"WHITE","starting_positions":[{"piece":"STONE","positions":["a","b"]}]},{"name":"BLACK","starting_positions":[]}]');
});

test("a map refuses a two-way link to a point that is not there", () => {
  assert.throws(() => mapBoard([mapPlace("a", [0, 0], 1)], [["a", "z"]]), /a link names z, which is not a point|not a place/);
});
