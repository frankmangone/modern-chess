import { normalizeSkin, pieceFace } from "@board/board-model.js";
import { Match, Switch } from "solid-js";

export interface PieceFaceProps {
  skin: Record<string, unknown> | null;
  code: string;
  player: string;
  players: string[];
}

/** A piece drawn the way the board draws it, so hands and captures match the game's skin. */
export function PieceFace(props: PieceFaceProps) {
  const face = () => pieceFace(normalizeSkin(props.skin), props.code, props.player, props.players);
  const colour = () =>
    face().colour === undefined
      ? undefined
      : face().kind === "fallback"
        ? { background: face().colour }
        : { color: face().colour };
  return (
    <Switch
      fallback={
        <span class={`island-piece mcb-${face().kind}`} style={colour()} aria-hidden="true">
          {face().value}
        </span>
      }
    >
      <Match when={face().kind === "svg"}>
        {/* Skins are authored with the games, like the specs, so their markup is trusted. */}
        <span class="island-piece mcb-svg" aria-hidden="true" innerHTML={face().value} />
      </Match>
      <Match when={face().kind === "image"}>
        <span class="island-piece mcb-image" aria-hidden="true">
          <img class="mcb-img" src={face().value} alt="" draggable={false} />
        </span>
      </Match>
    </Switch>
  );
}
