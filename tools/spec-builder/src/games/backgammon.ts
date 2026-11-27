// Backgammon: twenty-four points, two dice in each player's own bag, a die spent per move, a bar
// for hit checkers and bearing off into a tray.
import type { Game } from "../build.ts";
import { backgammonArt, FRAME } from "../art/backgammonArt.ts";
import { die, disc } from "../art/faces.ts";
import { allBornOff, backgammonZones, dieActions, dieLinks, pointPlaces, rollTwoDice, type Side } from "../recipes/backgammon.ts";
import { bagDef, endCheck, mapBoard, piece, players, standardSections, turns } from "../scaffold.ts";

const white: Side = { player: "WHITE", checker: "WHITE_CHECKER", label: "w", tray: "OFFW", bar: "White bar" };
const black: Side = { player: "BLACK", checker: "BLACK_CHECKER", label: "b", tray: "OFFB", bar: "Black bar" };

/** Where each side's checkers start, by point. */
const START = {
  WHITE_CHECKER: { 24: 2, 13: 5, 8: 3, 6: 5 },
  BLACK_CHECKER: { 1: 2, 12: 5, 17: 3, 19: 5 },
};

const board = mapBoard(
  pointPlaces(START, [[white.tray, [13, 0]], [black.tray, [13, 7]]], 15),
  dieLinks(white.label, black.label),
);

const howToPlay = [
  "Backgammon is a race with hitting. Each side has **fifteen checkers** and moves them all round the board and off it. **White** moves from point 24 down to point 1 and bears off from the home board (points 1 to 6); **Black** moves the other way, from point 1 up to 24, and bears off from 19 to 24. White rolls first.",
  "",
  "- **Roll.** Press *Roll* to throw two dice. Each die is a move of one checker that many points. A **double** is played four times.",
  "- **Moving.** Click the point a checker should go to. Several dice can reach the same point, and then the buttons ask which one to use. A point holding two or more enemy checkers is closed to you. A point with a single enemy checker (a *blot*) can be landed on: the blot is **hit** and goes to its owner's bar.",
  "- **The bar.** While you have a checker on your bar you play nothing else: enter it on the point that the die shows in the enemy's home board (a 1 enters on White's point 24, or Black's point 1, and so on). If you cannot, the turn passes.",
  "- **Bearing off.** When all your checkers are in your home board, click your tray to take one off with a die: the exact number, or a larger one when no checker stands farther back.",
  "- **Winning.** The first side to bear off all fifteen checkers wins.",
  "",
  "Left out: the doubling cube, and the extra points for gammons. A game is one win. In real Backgammon both dice must be played when they can. This game does not check that: it lets you play a die that leaves the other unplayable, and that die is lost, so look before you move.",
].join("\n");

export const backgammon: Game = {
  spec: {
    name: "BACKGAMMON",
    board,
    players: players("WHITE", "BLACK"),
    turns: turns(["WHITE", "BLACK"], ["PLAY"]),
    pieces: [white.checker, black.checker, ...Array.from({ length: 6 }, (_, i) => `D${i + 1}`)].map((code) => piece(code)),
    bags: [bagDef("DICE", { eachPlayer: true, capacity: 4 }), bagDef(white.bar), bagDef(black.bar)],
    zones: backgammonZones(white, black),
    actions: [rollTwoDice(), ...dieActions(white, black)],
    endChecks: [endCheck(allBornOff(15), "WIN")],
  },
  page: {
    slug: "backgammon",
    name: "Backgammon",
    tagline: "Race your fifteen checkers home on the roll of the dice, and hit the blots on the way.",
    description:
      "The oldest race game still played. Two dice move your checkers round twenty-four points to your home board, where you bear them off. Land on a lone enemy checker and it goes back to the bar; a point with two of your checkers is a wall.",
    tags: ["classic", "two players", "dice", "strategy"],
    accent: "#8c3b2a",
    skin: {
      captured_panel: false,
      hint: "Click where a checker should go.",
      board: { background: FRAME, art: backgammonArt() },
      // A point shows its checkers one after another, up to five, and the count on the last.
      stacks: { max: 5 },
      pieces: {
        WHITE_CHECKER: { svg: disc("#f3ecd8", "#8c7a52", "#ffffffaa") },
        BLACK_CHECKER: { svg: disc("#2b2622", "#0c0a09", "#ffffff33") },
        ...Object.fromEntries(Array.from({ length: 6 }, (_, i) => [`D${i + 1}`, { svg: die(i + 1) }])),
      },
    },
    sections: standardSections(howToPlay),
    featured: false,
    sortOrder: 21,
  },
  files: { spec: "crates/engine/specs/backgammon.json", page: "crates/game/pages/backgammon.json" },
};
