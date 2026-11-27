import test from "node:test";
import assert from "node:assert/strict";
import * as b from "../src/blocks.ts";

/** The block as the saved file has it, keys in order. */
const text = (block: unknown): string => JSON.stringify(block);

test("a bag names its owner only when it has one", () => {
  assert.equal(text(b.bag("TRAY")), '{"bag":"TRAY"}');
  assert.equal(text(b.bag("DICE", "MOVER")), '{"bag":"DICE","of":"MOVER"}');
});

test("regions write their keys in the order the specs do", () => {
  assert.equal(text(b.onBoard()), '{"region":"BOARD"}');
  assert.equal(text(b.onBoard(b.codes("RED_TOKEN"))), '{"region":"BOARD","where":{"codes":["RED_TOKEN"]}}');
  assert.equal(text(b.atBag("TRAY")), '{"region":"AT","at":{"bag":"TRAY"}}');
  assert.equal(text(b.atPlace("TARGET")), '{"region":"AT","at":{"place":"TARGET"}}');
  assert.equal(text(b.around("LAST", "jump")), '{"region":"AROUND","of":{"place":"LAST"},"through":"jump"}');
  assert.equal(
    text(b.follow("EACH", b.countOf(b.atBag("TRAY"), b.codes("PIP")), { through: "step" })),
    '{"region":"FOLLOW","from":{"place":"EACH"},"steps":{"count":{"in":{"region":"AT","at":{"bag":"TRAY"}},"where":{"codes":["PIP"]}}},"through":"step"}',
  );
  assert.equal(
    text(b.follow("TARGET", b.countOf(b.atPlace("TARGET")), { skipping: "OTHER_STORE" })),
    '{"region":"FOLLOW","from":{"place":"TARGET"},"steps":{"count":{"in":{"region":"AT","at":{"place":"TARGET"}}}},"skipping":"OTHER_STORE"}',
  );
});

test("a count test writes in, where, min and max in that order and only what is given", () => {
  const here = b.atPlace("EACH");
  assert.equal(text(b.count(here)), '{"test":"COUNT","in":{"region":"AT","at":{"place":"EACH"}}}');
  assert.equal(text(b.count(here, b.codes("X"), { min: 1 })), '{"test":"COUNT","in":{"region":"AT","at":{"place":"EACH"}},"where":{"codes":["X"]},"min":1}');
  assert.equal(text(b.count(here, undefined, { min: 0, max: 1 })), '{"test":"COUNT","in":{"region":"AT","at":{"place":"EACH"}},"min":0,"max":1}');
  assert.equal(text(b.count(here, b.codes("X"), { max: 1 })), '{"test":"COUNT","in":{"region":"AT","at":{"place":"EACH"}},"where":{"codes":["X"]},"max":1}');
});

test("a count test refuses bounds that cannot hold", () => {
  const here = b.atPlace("EACH");
  assert.throws(() => b.count(here, undefined, { min: 2, max: 1 }), /count: min 2 is above max 1/);
  assert.throws(() => b.count(here, undefined, { min: -1 }), /count: min must be 0 or more/);
  assert.throws(() => b.count(here, undefined, { max: 1.5 }), /count: max must be 0 or more/);
});

test("the other tests", () => {
  assert.equal(text(b.inZone("EACH", "FINISH")), '{"test":"IN_ZONE","at":{"place":"EACH"},"zone":"FINISH"}');
  assert.equal(text(b.playerIs("RED")), '{"test":"PLAYER","is":["RED"]}');
  assert.equal(text(b.stuck()), '{"test":"STUCK"}');
  assert.equal(text(b.not(b.stuck())), '{"test":"NOT","of":{"test":"STUCK"}}');
  assert.equal(text(b.all(b.stuck(), b.stuck())), '{"test":"ALL","of":[{"test":"STUCK"},{"test":"STUCK"}]}');
  assert.equal(text(b.any(b.stuck())), '{"test":"ANY","of":[{"test":"STUCK"}]}');
  assert.equal(text(b.some(b.onBoard(), b.stuck())), '{"test":"SOME","in":{"region":"BOARD"},"that":{"test":"STUCK"}}');
});

test("effects write their keys in the order the specs do", () => {
  const mine = b.codes("RED_TOKEN");
  assert.equal(text(b.remove(b.place("EACH"), 1, mine)), '{"effect":"REMOVE","at":{"place":"EACH"},"what":{"codes":["RED_TOKEN"]},"count":1}');
  assert.equal(text(b.remove(b.bag("TRAY"), "ALL")), '{"effect":"REMOVE","at":{"bag":"TRAY"},"count":"ALL"}');
  assert.equal(text(b.putDown(b.place("EACH"), "RED_TOKEN", 1)), '{"effect":"PLACE","at":{"place":"EACH"},"code":"RED_TOKEN","count":1}');
  assert.equal(
    text(b.move(b.place("TARGET"), b.place("EACH"), b.codes("STONE"), 1)),
    '{"effect":"MOVE","from":{"place":"TARGET"},"to":{"place":"EACH"},"what":{"codes":["STONE"]},"count":1}',
  );
  assert.equal(text(b.roll(b.bag("TRAY"), [{ PIP: 1 }])), '{"effect":"ROLL","at":{"bag":"TRAY"},"faces":[{"PIP":1}]}');
  assert.equal(text(b.roll(b.bag("DICE", "MOVER"), [{ D1: 1 }], 2)), '{"effect":"ROLL","at":{"bag":"DICE","of":"MOVER"},"dice":2,"faces":[{"D1":1}]}');
  assert.equal(text(b.stage("PLAY")), '{"effect":"STAGE","name":"PLAY"}');
  assert.equal(text(b.forEach(b.onBoard(), b.stage("PLAY"))), '{"effect":"FOR_EACH","in":{"region":"BOARD"},"do":[{"effect":"STAGE","name":"PLAY"}]}');
});

test("a condition has an else only when given one", () => {
  assert.equal(text(b.when(b.stuck(), [b.stage("A")])), '{"if":{"test":"STUCK"},"then":[{"effect":"STAGE","name":"A"}]}');
  assert.equal(
    text(b.when(b.stuck(), [b.stage("A")], [b.stage("B")])),
    '{"if":{"test":"STUCK"},"then":[{"effect":"STAGE","name":"A"}],"else":[{"effect":"STAGE","name":"B"}]}',
  );
});

test("an action names its stage only when it has one, before what it does", () => {
  assert.equal(text(b.action("ROLL", [b.stage("PLAY")])), '{"name":"ROLL","do":[{"effect":"STAGE","name":"PLAY"}]}');
  assert.equal(text(b.action("MOVE_1", [b.stage("PLAY")], "PLAY")), '{"name":"MOVE_1","stage":"PLAY","do":[{"effect":"STAGE","name":"PLAY"}]}');
});

test("a helper refuses a block that cannot be right, naming itself", () => {
  assert.throws(() => b.place(""), /place: a name is needed/);
  assert.throws(() => b.bag("TRAY", ""), /bag owner: a name is needed/);
  assert.throws(() => b.codes(), /codes: at least one piece code/);
  assert.throws(() => b.playerIs(), /playerIs: at least one player/);
  assert.throws(() => b.all(), /all: at least one test/);
  assert.throws(() => b.any(), /any: at least one test/);
  assert.throws(() => b.remove(b.place("EACH"), 0), /remove: the count must be 1 or more/);
  assert.throws(() => b.move(b.place("A"), b.place("B"), b.codes("X"), 1.5), /move: the count must be 1 or more/);
  assert.throws(() => b.putDown(b.place("A"), "X", -1), /place: the count must be 1 or more/);
  assert.throws(() => b.roll(b.bag("TRAY"), []), /roll: at least one face/);
  assert.throws(() => b.roll(b.bag("TRAY"), [{ A: 1 }], 0), /roll: dice must be 1 or more/);
  assert.throws(() => b.forEach(b.onBoard()), /forEach: it needs something to do/);
  assert.throws(() => b.when(b.stuck(), []), /when: it needs something to do/);
  assert.throws(() => b.action("A", []), /action A: it needs something to do/);
  assert.throws(() => b.stage(""), /stage: a name is needed/);
  assert.throws(() => b.around("LAST", ""), /link label: a name is needed/);
  assert.throws(() => b.follow("A", b.countOf(b.atPlace("A")), { skipping: "" }), /zone: a name is needed/);
});

test("a count that wants none says so with a min of 0 as well", () => {
  assert.throws(() => b.count(b.atBag("DECK"), undefined, { max: 0 }), /a max of 0 needs a min of 0 too/);
  assert.deepEqual(b.count(b.atBag("DECK"), undefined, { min: 0, max: 0 }), { test: "COUNT", in: { region: "AT", at: { bag: "DECK" } }, min: 0, max: 0 });
});

test("a slide is a chosen piece and a chosen move, and a placement writes no count", () => {
  assert.equal(JSON.stringify(b.choosePiece({ owner: "ALLY" })), '{"choose":"PIECE","where":{"owner":"ALLY"}}');
  assert.equal(JSON.stringify(b.chooseMove()), '{"choose":"MOVE"}');
  assert.equal(JSON.stringify(b.putOne(b.place(b.TARGET), "STONE")), '{"effect":"PLACE","at":{"place":"TARGET"},"code":"STONE"}');
});

test("a position repeats at two times or more", () => {
  assert.equal(JSON.stringify(b.repeats(3)), '{"test":"REPEATS","times":3}');
  assert.throws(() => b.repeats(1), /repeats: a position repeats at 2 times or more/);
});
