import { Show, createSignal } from "solid-js";
import type { Answer, BoardView, GameDetail } from "../api/types";
import { BoardPreview, type PreviewElement } from "./board-preview";
import { PlayIsland } from "./play-island";
import { previewSkin } from "./rules-text";

/**
 * "Try it" for a game with no board (cards): the engine runs in the page as it does for any game,
 * inside a board element nobody sees, and the page draws the same table the match page does. One
 * person plays every seat, each in turn, so a hidden hand or bid is shown to whoever plays now,
 * and "Try it" shows everything else by design too.
 */
export function TablePreview(props: { detail: GameDetail }) {
  const [view, setView] = createSignal<BoardView | null>(null);
  let engine: PreviewElement | undefined;
  const skin = () => previewSkin(props.detail);

  return (
    <div class="live-table">
      {/* The engine's own drawing is not shown or reached: the table below is what is played. */}
      <div class="live-table-engine" aria-hidden="true">
        <BoardPreview
          rules={props.detail.spec.rules}
          skin={skin()}
          onView={setView}
          onElement={(element) => (engine = element)}
        />
      </div>
      <Show when={view()}>
        {(current) => (
          <>
            <PlayIsland
              view={current()}
              skin={skin()}
              seat={current().turn}
              interactive={current().status.state === "playing"}
              finished={current().status.state !== "playing"}
              onChoose={(answer: Answer) => engine?.choose(answer)}
              onCancel={() => engine?.cancel()}
            />
            <button type="button" class="live-table-reset" onClick={() => engine?.reset()}>
              Start again
            </button>
          </>
        )}
      </Show>
    </div>
  );
}
