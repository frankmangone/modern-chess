// Builds every game and writes its files; with --check, writes nothing and fails when a file
// differs from what is built, so the committed JSON and the builder cannot drift apart.
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { build, toFile } from "./build.ts";
import { games } from "./games/index.ts";

const ROOT = fileURLToPath(new URL("../../../", import.meta.url));

const checking = process.argv.includes("--check");
const only = process.argv.slice(2).filter((arg) => !arg.startsWith("--"));
const differing: string[] = [];

for (const game of games) {
  if (only.length > 0 && !only.includes(game.page.slug)) continue;
  const built = await build(game);
  for (const [file, value] of [[game.files.spec, built.spec], [game.files.page, built.page]] as const) {
    const text = toFile(value);
    const path = `${ROOT}${file}`;
    let saved = "";
    try {
      saved = readFileSync(path, "utf8");
    } catch {
      // A file that does not exist yet differs from what is built.
    }
    if (saved === text) {
      console.log(`same     ${file}`);
    } else if (checking) {
      differing.push(file);
      console.log(`DIFFERS  ${file}`);
    } else {
      writeFileSync(path, text);
      console.log(`wrote    ${file}`);
    }
  }
}

if (differing.length > 0) {
  console.error(`\n${differing.length} file(s) differ from what the builder makes. Run \`make specs\` and commit the result, or fix the recipe.`);
  process.exit(1);
}
