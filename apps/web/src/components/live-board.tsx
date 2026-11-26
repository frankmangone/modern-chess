import { createSignal } from "solid-js";
import type { BoardView, GameDetail } from "../api/types";
import { BoardPreview } from "./board-preview";
import { DiceTray } from "./dice-tray";
import { showsBoard } from "./board-panel";
import { previewSkin } from "./rules-text";
import { TablePreview } from "./table-preview";

/** The game's board, playable in the page against yourself, with what the dice say under it. */
export function LiveBoard(props: { detail: GameDetail }) {
  const [view, setView] = createSignal<BoardView | null>(null);

  // A game with no board is tried on its table instead.
  if (!showsBoard(previewSkin(props.detail))) return <TablePreview detail={props.detail} />;

  return (
    <div class="live-board">
      <BoardPreview rules={props.detail.spec.rules} skin={previewSkin(props.detail)} onView={setView} />
      <DiceTray view={view()} />
    </div>
  );
}
