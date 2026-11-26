// Kalah, the common form of Mancala: six pits and a store each, four stones in every pit.
import type { Game } from "../build.ts";
import { ringEnds, ringLinks, ringPlaces, ringScore, ringZones, sow, AGAIN, type Ring } from "../recipes/sowing.ts";
import { mapBoard, piece, players, standardSections, turns } from "../scaffold.ts";

const kalah: Ring = {
  pits: 6,
  seeds: 4,
  stone: "STONE",
  south: { player: "SOUTH", prefix: "a", store: "sa" },
  north: { player: "NORTH", prefix: "b", store: "sb" },
};

const howToPlay = [
  "Kalah is the common form of Mancala. Each player has **six pits** in their row and a **store** at their end of the board, and every pit starts with four seeds.",
  "",
  "- **Sow.** On your turn, pick one of your pits and take every seed in it. Drop them one at a time into each place that follows, going round the board: your pits, your store, your opponent's pits, and on. You skip your opponent's store.",
  "- **Another turn.** If your last seed lands in your own store, you move again.",
  "- **Capture.** If your last seed lands in an empty pit of your own row, and the pit straight across the board holds seeds, you take them, and your last seed, into your store.",
  "- **The end.** The game ends as soon as either player has no seeds left in their pits. Each player's score is the seeds in their pits and their store, and the higher score wins.",
].join("\n");

export const kalahGame: Game = {
  spec: {
    name: "KALAH",
    board: mapBoard(ringPlaces(kalah), ringLinks(kalah)),
    players: players("SOUTH", "NORTH"),
    turns: turns(["SOUTH", "NORTH"], [AGAIN]),
    pieces: [piece("STONE")],
    zones: ringZones(kalah),
    actions: [sow(kalah)],
    endChecks: ringEnds(kalah),
    score: ringScore(kalah),
  },
  page: {
    slug: "kalah",
    name: "Kalah",
    tagline: "Sow your seeds round the board and capture across it.",
    description:
      "The common form of Mancala, for two. Six pits and a store each, four seeds in every pit. Pick up a pit and sow it, one seed to each place that follows, and bank more than your opponent.",
    tags: ["classic", "two players", "strategy"],
    accent: "#7a4e2d",
    skin: {
      captured_panel: false,
      pieces: {
        STONE: {
          svg: '<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><circle cx="50" cy="50" r="40" fill="#c9a66b" stroke="#5a3b1c" stroke-width="6"/><ellipse cx="38" cy="36" rx="10" ry="6" fill="#e6cf9d"/></svg>',
        },
      },
      board: { background: "#e6d3a8", lines: "#7a4e2d" },
    },
    sections: standardSections(howToPlay),
    featured: false,
    sortOrder: 17,
  },
  files: { spec: "crates/engine/specs/kalah.json", page: "crates/game/pages/kalah.json" },
};
