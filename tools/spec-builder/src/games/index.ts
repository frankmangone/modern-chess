import type { Game } from "../build.ts";
import { backgammon } from "./backgammon.ts";
import { goofspielGame } from "./goofspiel.ts";
import { kalahGame } from "./kalah.ts";
import { snakesAndLadders } from "./snakes-and-ladders.ts";

/** Every game the builder makes, in the order it makes them. */
export const games: Game[] = [snakesAndLadders, backgammon, kalahGame, goofspielGame];
