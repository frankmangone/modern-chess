// A blocking game on a graph, as Pong hau k'i is: two stones each on the points of a map, slid
// along its lines to an empty point, and a player who cannot move loses. Nothing is captured.
// The graph and where the stones begin are what a variant changes; a variant may begin with an
// empty board and have the players place their stones first (Do-guti).
import { atPlace, choosePiece, chooseMove, choosePlace, count, not, onBoard, place, putOne, repeats, stuck, TARGET, action, type Action } from "../blocks.ts";
import type { SpecParts } from "../build.ts";
import { endCheck, bothWays, mapBoard, mapPlace, playersAt, slider, turns } from "../scaffold.ts";

export interface BlockingGraph {
  name: string;
  /** The points, each with where it is drawn. */
  points: [name: string, x: number, y: number][];
  /** The lines between them; a stone slides either way along one. */
  lines: [string, string][];
  /** Where each side's two stones begin, or `"place"` for a board that begins empty, the players
   *  placing their stones one at a time before any is slid. */
  start: { WHITE: [string, string]; BLACK: [string, string] } | "place";
}

const STONE = "STONE";
const mine = { owner: "ALLY" };

/** The rules of the game on a graph: White moves first, three of the same position is a draw. */
export function blockingGame(graph: BlockingGraph): SpecParts {
  const { points, lines, start } = graph;
  const names = new Set(points.map(([name]) => name));
  if (names.size !== points.length) throw new Error(`blockingGame ${graph.name}: two points have the same name`);
  for (const [a, b] of lines) {
    for (const end of [a, b]) if (!names.has(end)) throw new Error(`blockingGame ${graph.name}: a line joins ${end}, which is not a point`);
  }
  const stones = start === "place" ? { WHITE: [], BLACK: [] } : start;
  if (start !== "place") {
    const used = [...start.WHITE, ...start.BLACK];
    for (const point of used) if (!names.has(point)) throw new Error(`blockingGame ${graph.name}: a stone starts on ${point}, which is not a point`);
    if (new Set(used).size !== used.length) throw new Error(`blockingGame ${graph.name}: two stones start on the same point`);
  }
  if (points.length < 5) throw new Error(`blockingGame ${graph.name}: four stones need at least five points to be able to move`);

  const slide: Action = action("MOVE", [...(start === "place" ? [count(onBoard(mine), undefined, { min: 2 })] : []), choosePiece(mine), chooseMove()]);
  const put: Action[] =
    start === "place"
      ? [action("PLACE", [count(onBoard(mine), undefined, { min: 0, max: 1 }), choosePlace(onBoard()), not(count(atPlace(TARGET))), putOne(place(TARGET), STONE)])]
      : [];

  return {
    name: graph.name,
    board: mapBoard(
      points.map(([name, x, y]) => mapPlace(name, [x, y], 1)),
      lines.map(([a, b]) => bothWays(a, b)),
    ),
    players: playersAt(STONE, stones),
    turns: turns(["WHITE", "BLACK"]),
    pieces: [slider(STONE)],
    actions: [...put, slide],
    endChecks: [endCheck(repeats(3), "DRAW"), endCheck(stuck(), "LOSE", { as: "NEXT" })],
  };
}
