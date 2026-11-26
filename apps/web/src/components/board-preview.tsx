import { Show, createSignal, onCleanup, onMount } from "solid-js";
import type { BoardView } from "../api/types";
import { loadBoardElement } from "./board-loader";

interface BoardPreviewProps {
  rules: unknown;
  skin: Record<string, unknown> | null;
  /** Told the board as it is, once the engine has set it up and after every move. A page that
   *  shows more than the board (the dice) learns what it needs from here. */
  onView?: (view: BoardView) => void;
  /** Given the element itself, for a page that draws the game and only needs the engine in it. */
  onElement?: (element: PreviewElement) => void;
}

/** What a page may ask of the board element in demo mode. */
export type PreviewElement = HTMLElement & {
  view?: BoardView;
  choose: (answer: unknown) => void;
  cancel: () => void;
  reset: () => void;
};

/** How often to ask a board that is still loading whether it is ready. */
const READY_POLL_MS = 150;
/** It gives up after about half a minute: a board that never loads has said so on its own. */
const MAX_READY_TRIES = 200;

/** The game's board in demo mode: playable against yourself in the page, with no server. The
 *  element reads the rules (the engine's spec) from its child and the skin from a URL; a data URL carries the skin
 *  the page already has. */
export function BoardPreview(props: BoardPreviewProps) {
  const [failed, setFailed] = createSignal(false);
  let board: PreviewElement | undefined;
  const skinUrl = () =>
    props.skin === null
      ? undefined
      : `data:application/json,${encodeURIComponent(JSON.stringify(props.skin))}`;

  /** The element draws itself again after every change, so a change shows up as new children. Until
   *  the engine is up there is no view to read, and asking throws: try again shortly. */
  function watch(element: PreviewElement) {
    const read = () => {
      try {
        const view = element.view;
        if (view) props.onView?.(view);
        return view !== undefined && view !== null;
      } catch {
        return false;
      }
    };
    const observer = new MutationObserver(read);
    observer.observe(element, { childList: true, subtree: true });
    let tries = 0;
    const poll = setInterval(() => (read() || ++tries >= MAX_READY_TRIES) && clearInterval(poll), READY_POLL_MS);
    onCleanup(() => {
      observer.disconnect();
      clearInterval(poll);
    });
  }

  onMount(() => {
    loadBoardElement().catch(() => setFailed(true));
    if (board && props.onView) watch(board);
    if (board) props.onElement?.(board);
  });

  return (
    <Show
      when={!failed()}
      fallback={<p class="feed-message" role="alert">The board preview could not be loaded.</p>}
    >
      <modern-chess-board ref={board} attr:skin={skinUrl()}>
        <script type="application/json" textContent={JSON.stringify(props.rules)} />
      </modern-chess-board>
    </Show>
  );
}
