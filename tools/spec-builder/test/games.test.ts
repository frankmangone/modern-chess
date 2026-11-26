// The drift check: every game is built again and compared, byte for byte, with the files that are
// committed. The committed JSON stays the source of truth; this keeps the builder honest to it.
import test from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { build, toFile } from "../src/build.ts";
import { games } from "../src/games/index.ts";

const root = new URL("../../../", import.meta.url);
const engineBuilt = existsSync(new URL("apps/docs/theme/pkg/modern_chess_wasm_bg.wasm", root));

for (const game of games) {
  test(`${game.page.slug}: the builder makes the committed spec and page, byte for byte`, { skip: engineBuilt ? false : "the engine is not built: run `make wasm`" }, async () => {
    const built = await build(game);
    assert.equal(toFile(built.spec), readFileSync(new URL(game.files.spec, root), "utf8"), game.files.spec);
    assert.equal(toFile(built.page), readFileSync(new URL(game.files.page, root), "utf8"), game.files.page);
  });
}

test("no two games write the same file", () => {
  const files = games.flatMap((game) => [game.files.spec, game.files.page]);
  assert.equal(new Set(files).size, files.length);
});
