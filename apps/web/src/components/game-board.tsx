import { Show, createSignal, onMount } from "solid-js";
import type { BoardView } from "../api/types";
import { loadBoardElement } from "./board-loader";

interface GameBoardProps {
  /** The board as a game starts, as the server reports it. */
  view: BoardView;
  skin: Record<string, unknown> | null;
}

/** A game's board as a picture: the starting position in the hero's frame, drawn by the shared
 *  board element with nothing to click. The element draws a view it is given without fetching the
 *  engine, so a page of these stays light. It shows what the name beside it already says, so it is
 *  hidden from assistive technology. */
export function GameBoard(props: GameBoardProps) {
  const [failed, setFailed] = createSignal(false);
  const skinUrl = () =>
    props.skin === null
      ? undefined
      : `data:application/json,${encodeURIComponent(JSON.stringify(props.skin))}`;

  onMount(() => {
    loadBoardElement().catch(() => setFailed(true));
  });

  return (
    <div class="game-board" data-testid="game-board" aria-hidden="true">
      <Show when={!failed()}>
        <modern-chess-board
          attr:remote={true}
          attr:skin={skinUrl()}
          prop:view={props.view}
          prop:interactive={false}
          prop:externalPrompts={true}
        />
      </Show>
    </div>
  );
}
