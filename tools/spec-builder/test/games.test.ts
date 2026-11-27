// The drift check: every game is built again and compared, byte for byte, with the files that are
// committed. The committed JSON stays the source of truth; this keeps the builder honest to it.
import test from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { build, filesOf, slugOf, toFile } from "../src/build.ts";
import { games } from "../src/games/index.ts";

const root = new URL("../../../", import.meta.url);
const engineBuilt = existsSync(new URL("apps/docs/theme/pkg/modern_chess_wasm_bg.wasm", root));

for (const game of games) {
  test(`${slugOf(game)}: the builder makes the committed files, byte for byte`, { skip: engineBuilt ? false : "the engine is not built: run `make wasm`" }, async () => {
    const built = await build(game);
    for (const [file, value] of filesOf(game, built)) assert.equal(toFile(value), readFileSync(new URL(file, root), "utf8"), file);
  });
}

test("no two games write the same file", () => {
  const files = games.flatMap((game) => [game.files.spec, ...(game.files.page === undefined ? [] : [game.files.page])]);
  assert.equal(new Set(files).size, files.length);
});

test("a game with a page and no page file, or the other way round, is refused", async () => {
  const [first] = games;
  assert.ok(first?.page);
  await assert.rejects(build({ ...first, files: { spec: first.files.spec } }, { validate: false }), /a page and a page file together, or neither/);
});
