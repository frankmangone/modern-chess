import { Show, createSignal, onMount } from "solid-js";
import { loadBoardElement } from "./board-loader";

interface BoardPreviewProps {
  rules: unknown;
  skin: Record<string, unknown> | null;
}

/** The game's board in demo mode: playable against yourself in the page, with no server. The
 *  element reads the rules (the engine's spec) from its child and the skin from a URL; a data URL carries the skin
 *  the page already has. */
export function BoardPreview(props: BoardPreviewProps) {
  const [failed, setFailed] = createSignal(false);
  const skinUrl = () =>
    props.skin === null
      ? undefined
      : `data:application/json,${encodeURIComponent(JSON.stringify(props.skin))}`;

  onMount(() => {
    loadBoardElement().catch(() => setFailed(true));
  });

  return (
    <Show
      when={!failed()}
      fallback={<p class="feed-message" role="alert">The board preview could not be loaded.</p>}
    >
      <modern-chess-board attr:skin={skinUrl()}>
        <script type="application/json" textContent={JSON.stringify(props.rules)} />
      </modern-chess-board>
    </Show>
  );
}
