import { For } from "solid-js";
import { PieceFace } from "./piece-face";

/** The pieces a game's skin shows off in its picture, as `"showcase": ["C01", "C13"]`: a game with
 *  no board (cards) has nothing else to put on its catalogue card. Anything but text is ignored. */
export function showcaseOf(skin: Record<string, unknown> | null | undefined): string[] {
  const list = skin?.showcase;
  return Array.isArray(list) ? list.filter((code): code is string => typeof code === "string") : [];
}

interface CardFanProps {
  skin: Record<string, unknown> | null;
  codes: string[];
  /** The seats, whose colours the cards take in turn. */
  players: string[];
}

/** The showcase cards fanned out like a hand, drawn in the game's skin. It is decoration, so it is
 *  hidden from assistive technology: the game's name and tagline say what it is. */
export function CardFan(props: CardFanProps) {
  const tilt = (index: number) => (index - (props.codes.length - 1) / 2) * 12;
  const lift = (index: number) => Math.abs(index - (props.codes.length - 1) / 2) * 2.5;
  return (
    <div class="card-fan" aria-hidden="true">
      <For each={props.codes}>
        {(code, index) => (
          <span
            class="card-fan-card"
            style={{ transform: `translateY(${lift(index())}cqw) rotate(${tilt(index())}deg)` }}
          >
            <PieceFace
              skin={props.skin}
              code={code}
              player={props.players[index() % Math.max(props.players.length, 1)] ?? ""}
              players={props.players}
            />
          </span>
        )}
      </For>
    </div>
  );
}
