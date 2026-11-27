// Variants of Pong hau k'i: Do-guti, which begins on an empty board, and two layouts of our own.
// Each is a spec and a page that names Pong hau k'i as its parent, so the front page lists it there.
import type { Game, PageParts } from "../build.ts";
import { blockingGame, type BlockingGraph } from "../recipes/blocking.ts";
import { standardSections } from "../scaffold.ts";

const stone = (fill: string, edge: string): string => `<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><circle cx="50" cy="50" r="42" fill="${fill}" stroke="${edge}" stroke-width="6"/></svg>`;

/** The look of Pong hau k'i itself: pale and dark stones on a parchment board. */
const skin = {
  captured_panel: false,
  pieces: { STONE: { WHITE: { svg: stone("#f6f1e4", "#4a3417") }, BLACK: { svg: stone("#2b2118", "#0f0a05") } } },
  board: { background: "#ecd9b0", lines: "#4a3417" },
};

/** The five points of Pong hau k'i: the corners of a square and its centre. */
const square: Pick<BlockingGraph, "points" | "lines"> = {
  points: [
    ["top-left", 0, 2],
    ["top-right", 2, 2],
    ["centre", 1, 1],
    ["bottom-left", 0, 0],
    ["bottom-right", 2, 0],
  ],
  lines: [
    ["top-left", "centre"],
    ["top-right", "centre"],
    ["bottom-left", "centre"],
    ["bottom-right", "centre"],
    ["top-left", "bottom-left"],
    ["top-right", "bottom-right"],
    ["bottom-left", "bottom-right"],
  ],
};

function variant(game: { slug: string; name: string; tagline: string; description: string; tags: string[]; accent: string; how: string; sortOrder: number; graph: BlockingGraph }): Game {
  const page: PageParts = {
    slug: game.slug,
    name: game.name,
    tagline: game.tagline,
    description: game.description,
    tags: game.tags,
    accent: game.accent,
    skin,
    sections: standardSections(game.how),
    parent: "pong-hau-ki",
    featured: false,
    sortOrder: game.sortOrder,
  };
  return { spec: blockingGame(game.graph), page, files: { spec: `crates/engine/specs/${game.slug.replaceAll("-", "_")}.json`, page: `crates/game/pages/${game.slug}.json` } };
}

export const doGuti = variant({
  slug: "do-guti",
  name: "Do-guti",
  tagline: "Place your two stones, then slide them until your opponent cannot move.",
  description:
    "A traditional blocking game for two, on the five points of Pong hau k'i. The board begins empty: each player places two stones, one at a time, and then the stones slide. Where you place them is half the game.",
  tags: ["classic", "two players", "strategy"],
  accent: "#9a6b2f",
  sortOrder: 23,
  how: [
    "Do-guti is played on the same five points as Pong hau k'i: the four corners of a square and its centre. Lines join each corner to the centre, and the left, right and bottom sides of the square. There is no line along the top. Each player has two stones, and the board begins empty.",
    "",
    "- **Place.** White puts a stone on any point, then Black, then White again, then Black. One point is left empty.",
    "- **Slide.** Once all four stones are down, White moves first. On your turn, slide one of your stones along a line to the empty point. Stones never jump and are never captured.",
    "- **Win** by leaving your opponent with no legal move. That can happen on the very first slide, so place with the empty point in mind.",
    "",
    "If the same position comes up three times, the game is a draw.",
  ].join("\n"),
  graph: { name: "DO-GUTI", ...square, start: "place" },
});

/** A pentagon of points with one more hanging from its top. */
export const spur = variant({
  slug: "pong-hau-ki-spur",
  name: "Pong hau k'i: Spur",
  tagline: "A pentagon with a spur: a new layout where one slip decides it.",
  description:
    "Pong hau k'i on a layout of our own: five points in a ring and a sixth hanging off the top. Two stones each and two empty points. With the best play nobody can force a win, but many of the positions that come up are already won or lost.",
  tags: ["variant", "two players", "strategy"],
  accent: "#6f5a2e",
  sortOrder: 24,
  how: [
    "This is Pong hau k'i on a layout that is not traditional: **five points in a ring**, and a **sixth** joined to the top of the ring by a single line.",
    "",
    "- Each player has two stones. White starts on the lower right and the left of the ring, Black on the lower left and the right, and the top of the ring and the spur are empty. White moves first.",
    "- On your turn, slide one of your stones along a line to an empty point. Stones never jump and are never captured.",
    "- **Win** by leaving your opponent with no legal move.",
    "",
    "If the same position comes up three times, the game is a draw.",
  ].join("\n"),
  graph: {
    name: "PONG HAU K'I: SPUR",
    points: [
      ["top", 2, 4],
      ["left", 0, 3],
      ["bottom-left", 1, 0],
      ["bottom-right", 3, 0],
      ["right", 4, 3],
      ["spur", 2, 6],
    ],
    lines: [
      ["top", "left"],
      ["left", "bottom-left"],
      ["bottom-left", "bottom-right"],
      ["bottom-right", "right"],
      ["right", "top"],
      ["top", "spur"],
    ],
    start: { WHITE: ["bottom-right", "left"], BLACK: ["bottom-left", "right"] },
  },
});

/** A hub joined to every other point, with an arc across four of them. */
export const fan = variant({
  slug: "pong-hau-ki-fan",
  name: "Pong hau k'i: Fan",
  tagline: "A hub with five spokes and an arc across: a layout of our own.",
  description:
    "Pong hau k'i on a layout of our own: a hub joined to every other point, and an arc across four of them. Two stones each and two empty points, so the hub is the place to hold.",
  tags: ["variant", "two players", "strategy"],
  accent: "#8b4a3a",
  sortOrder: 25,
  how: [
    "This is Pong hau k'i on a layout that is not traditional: a **hub** in the middle joined to the five points around it, and an **arc** that joins four of those points one to the next. The fifth, the foot, hangs below the hub.",
    "",
    "- Each player has two stones. White starts on the left and the far right of the arc, Black on the right and the far left, and the hub and the foot are empty. White moves first.",
    "- On your turn, slide one of your stones along a line to an empty point. Stones never jump and are never captured.",
    "- **Win** by leaving your opponent with no legal move.",
    "",
    "If the same position comes up three times, the game is a draw.",
  ].join("\n"),
  graph: {
    name: "PONG HAU K'I: FAN",
    points: [
      ["hub", 2, 2],
      ["far-left", 0, 3],
      ["left", 1, 4],
      ["right", 3, 4],
      ["far-right", 4, 3],
      ["foot", 2, 0],
    ],
    lines: [
      ["hub", "far-left"],
      ["hub", "left"],
      ["hub", "right"],
      ["hub", "far-right"],
      ["hub", "foot"],
      ["far-left", "left"],
      ["left", "right"],
      ["right", "far-right"],
    ],
    start: { WHITE: ["left", "far-right"], BLACK: ["right", "far-left"] },
  },
});
