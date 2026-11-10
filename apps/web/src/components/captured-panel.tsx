import { For, Show } from "solid-js";
import type { BoardView } from "../api/types";
import { PieceFace } from "./piece-face";
import { seatColour } from "./seat-colour";

interface CapturedPanelProps {
  view: BoardView;
  skin: Record<string, unknown> | null;
}

/** One entry per piece, so a side that lost three pawns gets three. */
function expand(lost: Record<string, number>): string[] {
  return Object.entries(lost).flatMap(([code, count]) => Array<string>(count).fill(code));
}

function summary(lost: Record<string, number>): string {
  return Object.entries(lost)
    .map(([code, count]) => `${code} ×${count}`)
    .join(", ");
}

/** Who is ahead on material, for a game with two sides and values for its pieces. */
function standing(view: BoardView): string | null {
  const [first, second] = view.players;
  if (view.players.length !== 2 || first === undefined || second === undefined) return null;
  const a = view.material[first];
  const b = view.material[second];
  if (a === undefined || b === undefined) return null;
  if (a === b) return "= 0";
  return a > b ? `${first} +${a - b}` : `${second} +${b - a}`;
}

/** The pieces each side has lost, with the material count under them. */
export function CapturedPanel(props: CapturedPanelProps) {
  const lost = (seat: string) => props.view.captured[seat] ?? {};
  const total = () =>
    props.view.players.reduce((sum, seat) => sum + expand(lost(seat)).length, 0);

  return (
    <aside class="captured-panel" aria-label="Captured pieces">
      <header class="captured-head">
        <span class="captured-label">CAPTURED</span>
        <span class="captured-count" aria-label={`Pieces lost: ${total()} in total`}>
          {total()}
        </span>
      </header>
      <For each={props.view.players}>
        {(seat) => (
          <section class="captured-box">
            <p class="captured-owner">
              <span class="captured-dot" style={{ background: seatColour(props.view.players, seat) }} />
              {seat}
            </p>
            <div class="captured-pieces" aria-label={`${seat} lost: ${summary(lost(seat))}`}>
              <Show
                when={expand(lost(seat)).length > 0}
                fallback={<span class="captured-empty">Nothing lost yet</span>}
              >
                <For each={expand(lost(seat))}>
                  {(code) => (
                    <PieceFace skin={props.skin} code={code} player={seat} players={props.view.players} />
                  )}
                </For>
              </Show>
            </div>
          </section>
        )}
      </For>
      <Show when={standing(props.view)}>
        {(text) => (
          <div class="captured-material">
            <span>Material</span>
            <strong>{text()}</strong>
          </div>
        )}
      </Show>
    </aside>
  );
}
