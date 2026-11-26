// Snakes and Ladders: a hundred squares, ladders and snakes as jump links, a die thrown into a
// shared bag, and a walk that nothing chooses.
import type { Game } from "../build.ts";
import { action, atBag, bag, codes, countOf } from "../blocks.ts";
import { trackArt } from "../art/trackArt.ts";
import { countFaces, throwIntoBag } from "../recipes/dice.ts";
import { firstInZone, walkAndJump } from "../recipes/race.ts";
import { allJumps, jumpLinks, stepLinks, trackPlaces } from "../recipes/track.ts";
import { bagDef, endCheck, mapBoard, piece, players, standardSections, turns, zone } from "../scaffold.ts";

const SQUARES = 100;
const ROW_LENGTH = 10;
const LADDERS = new Map([[1, 38], [4, 14], [9, 31], [21, 42], [28, 84], [36, 44], [51, 67], [71, 91], [80, 100]]);
const SNAKES = new Map([[16, 6], [47, 26], [49, 11], [56, 53], [62, 19], [64, 60], [87, 24], [93, 73], [95, 75], [98, 78]]);
const JUMPS = allJumps(LADDERS, SNAKES);

const board = mapBoard(
  trackPlaces({ squares: SQUARES, rowLength: ROW_LENGTH, capacity: 2, pad: { at: [-1, 0], start: { RED_TOKEN: 1, BLUE_TOKEN: 1 } } }),
  [...stepLinks(SQUARES, "step"), ...jumpLinks(JUMPS, "jump")],
);

/** How far a throw takes a token: the pips the throw left in the tray. */
const pips = countOf(atBag("TRAY"), codes("PIP"));
const race = (side: string) => walkAndJump({ player: side, token: `${side}_TOKEN`, steps: pips, step: "step", jump: "jump" });

const token = (fill: string, edge: string) =>
  `<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><circle cx="50" cy="50" r="42" fill="${fill}" stroke="${edge}" stroke-width="8"/><ellipse cx="38" cy="36" rx="12" ry="7" fill="#ffffff55"/></svg>`;
const pip =
  '<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg"><rect x="8" y="8" width="84" height="84" rx="16" fill="#fbf6ea" stroke="#5a4326" stroke-width="6"/><circle cx="50" cy="50" r="16" fill="#3b2a14"/></svg>';

const howToPlay =
  "Snakes and Ladders is a race along a numbered board of **100 squares**. Both players start off the board, and **Red** rolls first.\n\n" +
  "- **Roll.** On your turn, roll the die and move your token that many squares, following the numbers: left to right on the bottom row, then right to left on the next, and so on up the board.\n" +
  "- **Ladders.** If you end on the foot of a ladder, climb it to the square at its top.\n" +
  "- **Snakes.** If you end on a snake's head, slide down to the square at its tail.\n" +
  "- **Sharing.** Both tokens may stand on the same square. Nothing is captured.\n" +
  "- **Winning.** The first token to land **exactly** on square 100 wins. If your roll would take you past 100, you stay where you are and the turn passes.\n\n" +
  "Nothing is chosen in this game: every move is the roll, so luck decides it.";

export const snakesAndLadders: Game = {
  spec: {
    name: "SNAKES AND LADDERS",
    board,
    players: players("RED", "BLUE"),
    turns: turns(["RED", "BLUE"]),
    pieces: [piece("RED_TOKEN"), piece("BLUE_TOKEN"), piece("PIP")],
    bags: [bagDef("TRAY")],
    zones: [zone("FINISH", { RED: [String(SQUARES)], BLUE: [String(SQUARES)] })],
    actions: [action("ROLL", [...throwIntoBag(bag("TRAY"), countFaces("PIP")), race("RED"), race("BLUE")])],
    endChecks: [endCheck(firstInZone("FINISH"), "WIN")],
  },
  page: {
    slug: "snakes-and-ladders",
    name: "Snakes and Ladders",
    tagline: "Race up the board on the roll of a die; ladders lift you, snakes drop you.",
    description:
      "A race game for two on a numbered board of a hundred squares. Roll a die and move that many squares. Climb a ladder to jump ahead, slide down a snake to fall back, and land exactly on 100 to win.",
    tags: ["classic", "two players", "dice", "luck"],
    accent: "#3f9a4a",
    skin: {
      captured_panel: false,
      board: {
        background: "#e9d9ae",
        art: trackArt({ squares: SQUARES, rowLength: ROW_LENGTH, ladders: LADDERS, snakes: SNAKES, snakeColours: ["#3f9a4a", "#7a4fb0", "#c2543a", "#2f8f9a", "#b8913a"] }),
      },
      pieces: {
        RED_TOKEN: { svg: token("#c0392b", "#5e1610") },
        BLUE_TOKEN: { svg: token("#2f5fa8", "#10254a") },
        PIP: { svg: pip },
      },
    },
    sections: standardSections(howToPlay),
    featured: false,
    sortOrder: 20,
  },
  files: { spec: "crates/engine/specs/snakes_and_ladders.json", page: "crates/game/pages/snakes-and-ladders.json" },
};
