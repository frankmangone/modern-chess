// Turns the catalogue's page files into the data these pages read: data/games.json and one skin
// file per game. Run from anywhere: node pocs/e-quiet-cards/build-data.mjs
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pages = join(here, "../../crates/game/pages");
mkdirSync(join(here, "data/skins"), { recursive: true });
mkdirSync(join(here, "data/views"), { recursive: true });

// A card draws its board from the view a game starts with, as the real app does: no engine runs
// on the page. The views come from the same wasm build, here in Node.
const pkg = join(here, "../../apps/docs/theme/pkg");
const wasm = await import(pathToFileURL(join(pkg, "modern_chess_wasm.js")).href);
await wasm.default({ module_or_path: readFileSync(join(pkg, "modern_chess_wasm_bg.wasm")) });

// The kinds the list is organised by (a game appears once; a variant sits under its parent).
const KINDS = [
  { id: "chess", title: "Chess and its variants", slugs: ["chess"] },
  { id: "elsewhere", title: "Chess from other worlds", slugs: ["shogi", "xiangqi", "chaturanga", "shatranj"] },
  { id: "territory", title: "Place, surround, connect", slugs: ["go", "hex", "ataxx"] },
  { id: "dice", title: "Race and dice", slugs: ["backgammon", "snakes-and-ladders"] },
  { id: "small", title: "Small and old", slugs: ["checkers", "kalah", "pong-hau-ki", "fox-and-geese"] },
];

// Sample play counts for this week, to show how the popular band is chosen. The real app would
// count lobbies and matches; nothing here is measured.
const PLAYS = { chess: 1284, go: 702, kalah: 655, backgammon: 541, checkers: 498, "snakes-and-ladders": 433, hex: 310, shogi: 288, xiangqi: 265, ataxx: 190, "pong-hau-ki": 120, chaturanga: 96, shatranj: 77, "fox-and-geese": 64 };
const titled = (slug) => slug.split("-").map((w) => w[0].toUpperCase() + w.slice(1)).join(" ");
const games = [];
for (const file of readdirSync(pages).filter((f) => f.endsWith(".json"))) {
  const page = JSON.parse(readFileSync(join(pages, file), "utf8"));
  const howTo = page.sections.find((s) => s.type === "how_to_play")?.markdown ?? "";
  writeFileSync(join(here, "data/skins", `${page.slug}.json`), JSON.stringify(page.skin ?? {}));
  const specFile = join(here, "../../crates/engine/specs", `${page.slug.replaceAll("-", "_")}.json`);
  writeFileSync(join(here, "data/views", `${page.slug}.json`), new wasm.Board(readFileSync(specFile, "utf8")).view());
  games.push({
    slug: page.slug,
    name: page.name ?? titled(page.slug),
    tagline: page.tagline,
    description: page.description,
    tags: page.tags,
    accent: page.accent,
    parent: page.parent ?? null,
    featured: page.featured,
    plays: PLAYS[page.slug] ?? 0,
    order: page.sort_order,
    spec: `../../crates/engine/specs/${page.slug.replaceAll("-", "_")}.json`,
    skin: `data/skins/${page.slug}.json`,
    howTo,
  });
}
games.sort((a, b) => a.order - b.order);
writeFileSync(join(here, "data/games.json"), JSON.stringify({ kinds: KINDS, games }, null, 1));
console.log(`${games.length} games`);
