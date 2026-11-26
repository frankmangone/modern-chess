// Turns the catalogue's page files into the data these pages read: data/games.json and one skin
// file per game. Run from anywhere: node pocs/d-play-first/build-data.mjs
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pages = join(here, "../../crates/game/pages");
mkdirSync(join(here, "data/skins"), { recursive: true });

// The kinds the list is organised by (a game appears once; a variant sits under its parent).
const KINDS = [
  { id: "chess", title: "Chess and its variants", slugs: ["chess"] },
  { id: "elsewhere", title: "Chess from other worlds", slugs: ["shogi", "xiangqi", "chaturanga", "shatranj"] },
  { id: "territory", title: "Place, surround, connect", slugs: ["go", "hex", "ataxx"] },
  { id: "dice", title: "Race and dice", slugs: ["backgammon", "snakes-and-ladders"] },
  { id: "small", title: "Small and old", slugs: ["checkers", "kalah", "pong-hau-ki", "fox-and-geese"] },
];

const titled = (slug) => slug.split("-").map((w) => w[0].toUpperCase() + w.slice(1)).join(" ");
const games = [];
for (const file of readdirSync(pages).filter((f) => f.endsWith(".json"))) {
  const page = JSON.parse(readFileSync(join(pages, file), "utf8"));
  const howTo = page.sections.find((s) => s.type === "how_to_play")?.markdown ?? "";
  writeFileSync(join(here, "data/skins", `${page.slug}.json`), JSON.stringify(page.skin ?? {}));
  games.push({
    slug: page.slug,
    name: page.name ?? titled(page.slug),
    tagline: page.tagline,
    description: page.description,
    tags: page.tags,
    accent: page.accent,
    parent: page.parent ?? null,
    featured: page.featured,
    order: page.sort_order,
    spec: `../../crates/engine/specs/${page.slug.replaceAll("-", "_")}.json`,
    skin: `data/skins/${page.slug}.json`,
    howTo,
  });
}
games.sort((a, b) => a.order - b.order);
writeFileSync(join(here, "data/games.json"), JSON.stringify({ kinds: KINDS, games }, null, 1));
console.log(`${games.length} games`);
