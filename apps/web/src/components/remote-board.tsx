import { Show, createSignal, onMount } from "solid-js";
import type { Answer, BoardView, Command } from "../api/types";
import { loadBoardElement } from "./board-loader";

interface RemoteBoardProps {
  view: BoardView;
  skin: Record<string, unknown> | null;
  interactive: boolean;
  /** Draw the board from the second player's side. */
  flipped: boolean;
  onCommand: (command: Command) => void;
}

/** The answer the board element put in a `choose` event: one place, code, hand piece or action. */
function answerOf(event: Event): Answer {
  return (event as CustomEvent<Answer>).detail;
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
        prop:externalPrompts={true}
        on:choose={(event) => props.onCommand({ type: "choose", ...answerOf(event) })}
        on:cancel={() => props.onCommand({ type: "cancel" })}
      />
    </Show>
  );
}
