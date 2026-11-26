import { For, Show, createEffect, createMemo, createSignal, on } from "solid-js";
import type { BoardView } from "../api/types";
import { seatColour } from "./seat-colour";

/** What the dice of a game say right now. */
export interface DiceReading {
  /** The faces showing; none when the dice wait to be rolled. */
  faces: number[];
  /** The seat the dice belong to, whose colour tints them. */
  seat: string;
  /** The bold line: who rolled, or who is to roll. */
  headline: string;
  /** The quiet line under it. */
  note: string;
  /** Changes when the faces do, so the tray can tell a new roll from a redraw. */
  key: string;
}

/** The dice a game keeps are in its bags, by a convention of the specs: each player's `DICE` bag
 *  holds the faces they rolled (D1 to D6, a count each), and a shared `TRAY` bag holds a `PIP`
 *  for each pip of the one die last rolled. A game with neither has nothing to show. */
const DICE_BAG = "DICE";
const TRAY_BAG = "TRAY";
const DIE_CODE = /^D([1-6])$/;
const PIP = "PIP";

/** "RED" reads "Red". */
function nice(seat: string): string {
  return seat.charAt(0).toUpperCase() + seat.slice(1).toLowerCase();
}

/** The player whose turn came before the given one: the one who rolled the die in the tray. */
function previous(view: BoardView): string {
  const at = view.players.indexOf(view.turn);
  return view.players[(at - 1 + view.players.length) % view.players.length] ?? view.turn;
}

/** What the tray shows for this view, or null for a game without dice. */
export function readDice(view: BoardView): DiceReading | null {
  const own = view.bags.filter((bag) => bag.name === DICE_BAG);
  if (own.length > 0) {
    const mine = own.find((bag) => bag.owner === view.turn);
    const faces = Object.entries(mine?.pieces ?? {}).flatMap(([code, count]) => {
      const face = DIE_CODE.exec(code)?.[1];
      return face === undefined ? [] : Array<number>(count).fill(Number(face));
    });
    const key = `${view.turn}:${faces.join("")}`;
    if (faces.length === 0) {
      return { faces, seat: view.turn, headline: `${nice(view.turn)} to roll`, note: "Roll below the board", key };
    }
    return {
      faces,
      seat: view.turn,
      headline: `${nice(view.turn)}’s dice`,
      note: faces.length > 2 ? "A double: four moves" : "Each die is one move",
      key,
    };
  }
  const tray = view.bags.find((bag) => bag.name === TRAY_BAG);
  if (tray === undefined) return null;
  const pips = tray.pieces[PIP] ?? 0;
  if (pips === 0) {
    return {
      faces: [],
      seat: view.turn,
      headline: `${nice(view.turn)} to roll`,
      note: "Roll below the board",
      key: `${view.turn}:0`,
    };
  }
  const roller = previous(view);
  return {
    faces: [Math.min(pips, 6)],
    seat: roller,
    headline: `${nice(roller)} rolled ${pips}`,
    note: `${nice(view.turn)} to roll`,
    key: `${view.turn}:${pips}`,
  };
}

const PIPS: Record<number, number[]> = {
  1: [4],
  2: [0, 8],
  3: [0, 4, 8],
  4: [0, 2, 6, 8],
  5: [0, 2, 4, 6, 8],
  6: [0, 2, 3, 5, 6, 8],
};

const NINE = [0, 1, 2, 3, 4, 5, 6, 7, 8];

function Die(props: { value: number; tint: string; tumble: boolean }) {
  return (
    <div
      class="die"
      classList={{ tumble: props.tumble }}
      style={{ "--die": `color-mix(in srgb, ${props.tint} 30%, #f7ecd5)` }}
      role="img"
      aria-label={`A die showing ${props.value}`}
    >
      <For each={NINE}>{(cell) => <i classList={{ on: PIPS[props.value]?.includes(cell) }} />}</For>
    </div>
  );
}

interface DiceTrayProps {
  /** The board as it is now; null until there is one. */
  view: BoardView | null;
}

/** What was rolled, kept in view until the next roll: the faces, whose they are, and what comes
 *  next. It draws nothing for a game without dice. */
export function DiceTray(props: DiceTrayProps) {
  const reading = createMemo(() => (props.view === null ? null : readDice(props.view)));
  const [fresh, setFresh] = createSignal(false);
  // A new roll tumbles in; the same faces drawn again, or the first look at the page, do not.
  createEffect(
    on(
      () => reading()?.key,
      (key, before) => setFresh(before !== undefined && key !== before && (reading()?.faces.length ?? 0) > 0),
    ),
  );

  return (
    <Show when={reading()}>
      {(current) => (
        <div class="dice-tray" data-testid="dice-tray">
          <div class="dice-faces">
            {/* Keyed by the roll, so each new one is drawn afresh and tumbles in. */}
            <Show when={current().key} keyed>
              <Show
                when={current().faces.length > 0}
                fallback={<div class="die ghost" aria-hidden="true">?</div>}
              >
                <For each={current().faces}>
                  {(face) => (
                    <Die
                      value={face}
                      tint={seatColour(props.view?.players ?? [], current().seat)}
                      tumble={fresh()}
                    />
                  )}
                </For>
              </Show>
            </Show>
          </div>
          <div class="dice-text" aria-live="polite">
            <b>{current().headline}</b>
            <span>{current().note}</span>
          </div>
        </div>
      )}
    </Show>
  );
}
