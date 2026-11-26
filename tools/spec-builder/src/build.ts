// Turns a game described with recipes into the two plain objects that are saved: the spec the
// engine reads and the page the hub shows. Only these are saved: nothing records that recipes
// were used, and the engine never hears of them.
import type { Action } from "./blocks.ts";
import { readSpec } from "./engine.ts";
import type { BagDef, EndCheck, MapBoard, PieceDef, PlayerDef, ScoreTerm, Section, TurnsDef, ZoneDef } from "./scaffold.ts";

export interface SpecParts {
  name: string;
  board: MapBoard;
  players: PlayerDef[];
  turns: TurnsDef;
  pieces: PieceDef[];
  bags?: BagDef[];
  zones?: ZoneDef[];
  actions: Action[];
  endChecks: EndCheck[];
  score?: ScoreTerm[];
}

export interface PageParts {
  slug: string;
  name: string;
  tagline: string;
  description: string;
  tags: string[];
  accent: string;
  /** What the board shows: its pictures, colours and hints, as the page's JSON has them. */
  skin: Record<string, unknown>;
  sections: Section[];
  featured: boolean;
  sortOrder: number;
}

/** A game: its rules, its page, and the files it is saved in, relative to the repository. */
export interface Game {
  spec: SpecParts;
  page: PageParts;
  files: { spec: string; page: string };
}

export interface Built {
  spec: Record<string, unknown>;
  page: Record<string, unknown>;
}

/** Keys of a spec in the order the committed ones have them. */
function specOf(parts: SpecParts): Record<string, unknown> {
  const spec: Record<string, unknown> = { format_version: 1, name: parts.name, board: parts.board, players: parts.players, turns: parts.turns, pieces: parts.pieces };
  if (parts.bags !== undefined) spec.bags = parts.bags;
  if (parts.zones !== undefined) spec.zones = parts.zones;
  spec.actions = parts.actions;
  spec.end_checks = parts.endChecks;
  if (parts.score !== undefined) spec.score = parts.score;
  return spec;
}

function pageOf(parts: PageParts): Record<string, unknown> {
  return {
    slug: parts.slug,
    name: parts.name,
    tagline: parts.tagline,
    description: parts.description,
    tags: parts.tags,
    accent: parts.accent,
    skin: parts.skin,
    sections: parts.sections,
    featured: parts.featured,
    sort_order: parts.sortOrder,
  };
}

/** Builds a game. The spec is then read by the engine, so a spec it refuses is refused here with
 *  its message; `validate: false` is for tests of pieces that are not whole games. */
export async function build(game: Game, options: { validate?: boolean } = {}): Promise<Built> {
  const spec = specOf(game.spec);
  if (options.validate !== false) await readSpec(game.spec.name, spec);
  return { spec, page: pageOf(game.page) };
}

/** How a built object is saved: two-space JSON and a final newline. */
export function toFile(value: unknown): string {
  return `${JSON.stringify(value, null, 2)}\n`;
}
