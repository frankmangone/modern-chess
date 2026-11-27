import { Show, createSignal } from "solid-js";
import type { BoardView } from "../api/types";
import { BoardPreview } from "../components/board-preview";
import { DiceTray } from "../components/dice-tray";

interface PlaygroundProps {
  rules: unknown;
  skin: Record<string, unknown> | null;
  /** Told each time the board draws itself: it is up, so what was run did not hang the page. */
  onAlive: () => void;
}

/** The game as written, playable in the page against yourself, with the dice under it, and for
 *  the curious the position as the engine holds it. */
export function Playground(props: PlaygroundProps) {
  const [view, setView] = createSignal<BoardView | null>(null);
  const [open, setOpen] = createSignal(false);

  return (
    <div class="live-board builder-board">
      <BoardPreview
        rules={props.rules}
        skin={props.skin}
        onView={(next) => {
          setView(next);
          props.onAlive();
        }}
      />
      <DiceTray view={view()} />
      <details class="builder-state" onToggle={(event) => setOpen(event.currentTarget.open)}>
        <summary>What the engine sees</summary>
        <Show when={open() && view()}>
          <pre>{JSON.stringify(view(), null, 2)}</pre>
        </Show>
      </details>
    </div>
  );
}
