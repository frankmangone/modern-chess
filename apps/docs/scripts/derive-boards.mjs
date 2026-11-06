// Builds small, prepared positions for the docs from the real game specs.
//
// A recipe in `boards.derived.json` names a base spec in `crates/engine/specs/` and says what to
// replace: the board, the zones, and where each player's pieces start. Everything else (the pieces,
// their moves, the rules) comes straight from the base spec, so a prepared board can never drift
// from the game it teaches. Output goes to `src/boards/<out>.json` (generated, git-ignored).
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export function derive(recipe, base) {
  const spec = structuredClone(base);
  const label = recipe.out ?? recipe.base;

  if (recipe.name) spec.name = recipe.name;
  if (recipe.board) spec.board = recipe.board;

  for (const [code, check] of Object.entries(recipe.zones ?? {})) {
    const condition = spec.conditions?.find((c) => c.code === code);
    if (!condition) throw new Error(`${label}: base ${recipe.base} has no condition ${code}`);
    condition.check = check;
  }

  const known = new Set(spec.players.map((p) => p.name));
  for (const name of Object.keys(recipe.players ?? {})) {
    if (!known.has(name)) throw new Error(`${label}: unknown player ${name}`);
  }
  for (const player of spec.players) {
    const layout = recipe.players?.[player.name];
    if (!layout) throw new Error(`${label}: no layout for player ${player.name}`);
    player.starting_positions = layout.map(([piece, positions]) => ({ piece, positions }));
  }
  return spec;
}

function main() {
  const here = dirname(fileURLToPath(import.meta.url));
  const docs = join(here, "..");
  const specs = join(docs, "..", "..", "crates", "engine", "specs");
  const out = join(docs, "src", "boards");
  mkdirSync(out, { recursive: true });

  const recipes = JSON.parse(readFileSync(join(docs, "boards.derived.json"), "utf8"));
  for (const recipe of recipes) {
    const base = JSON.parse(readFileSync(join(specs, `${recipe.base}.json`), "utf8"));
    writeFileSync(join(out, `${recipe.out}.json`), JSON.stringify(derive(recipe, base), null, 2) + "\n");
    console.log(`derived ${recipe.out} from ${recipe.base}`);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main();
