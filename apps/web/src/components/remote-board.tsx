import { Show, createSignal, onMount } from "solid-js";
import type { BoardView, Command } from "../api/types";
import { loadBoardElement } from "./board-loader";

interface Square {
  x: number;
  y: number;
}

interface RemoteBoardProps {
  view: BoardView;
  skin: Record<string, unknown> | null;
  interactive: boolean;
  /** Draw the board from the second player's side. */
  flipped: boolean;
  onCommand: (command: Command) => void;
}

/** What the board element put in the event: `{x, y}` or `{code}`. */
function detailOf<T>(event: Event): T {
  return (event as CustomEvent<T>).detail;
}

/** The match's board: it draws the view the server gave and reports clicks. The rules live
 *  on the server; this only passes things along. */
export function RemoteBoard(props: RemoteBoardProps) {
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
      fallback={<p class="feed-message" role="alert">The board could not be loaded.</p>}
    >
      <modern-chess-board
        attr:remote={true}
        attr:skin={skinUrl()}
        prop:view={props.view}
        prop:interactive={props.interactive}
        prop:flipped={props.flipped}
        on:select={(event) => props.onCommand({ type: "select", ...detailOf<Square>(event) })}
        on:move={(event) => props.onCommand({ type: "move", ...detailOf<Square>(event) })}
        on:promote={(event) => props.onCommand({ type: "promote", code: detailOf<{ code: string }>(event).code })}
      />
    </Show>
  );
}
