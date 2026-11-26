// A race along a track on the throw of a die: nothing is chosen, so every move is the throw.
import { EACH, LAST, all, around, atPlace, codes, count, follow, forEach, inZone, onBoard, place, playerIs, putDown, remove, some, when, type Step, type Steps, type Test } from "../blocks.ts";

export interface RaceOptions {
  /** The player whose turn this is for. */
  player: string;
  /** The piece that player races. */
  token: string;
  /** How far to go: a count of pieces in a bag, say, that the throw has put there. */
  steps: Steps;
  /** The label of the links a step follows, and of the ones a jump follows. */
  step: string;
  jump: string;
}

/** The player's token walks as many steps as thrown; if it lands where a jump starts, it takes it.
 *  A walk that would run past the end of the track is not taken, so the token stays where it is. */
export function walkAndJump(options: RaceOptions): Step {
  const { player, token, steps, step, jump } = options;
  const mine = codes(token);
  const walk = follow(EACH, steps, { through: step });
  return when(playerIs(player), [
    forEach(
      onBoard(mine),
      when(count(walk, undefined, { min: 0 }), [
        remove(place(EACH), 1, mine),
        forEach(walk, remove(place(LAST), 1, mine), putDown(place(EACH), token, 1)),
        forEach(around(LAST, jump), remove(place(LAST), 1, mine), putDown(place(EACH), token, 1)),
      ]),
    ),
  ]);
}

/** The game is won when some token stands in the zone: every square of the board that is in the
 *  zone and holds a piece of its owner's. */
export function firstInZone(zoneName: string): Test {
  return some(onBoard(), all(inZone(EACH, zoneName), count(atPlace(EACH), { owner: "ALLY" })));
}
