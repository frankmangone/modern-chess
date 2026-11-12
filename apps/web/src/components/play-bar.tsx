import { Show, type JSX } from "solid-js";

interface PlayBarProps {
  seat: string;
  /** Who sits there, or null while the seat is empty. */
  handle: string | null;
  /** The viewer is this player. */
  you: boolean;
  /** It is this player's move. */
  active: boolean;
  /** Room on the right for something to do with this seat, such as taking it. */
  children?: JSX.Element;
}

/** A strip above or below the board naming the player who sits on that side. */
export function PlayBar(props: PlayBarProps) {
  return (
    <div class="play-bar" classList={{ "play-bar-self": props.you, "play-bar-turn": props.active }}>
      <div class="play-bar-who">
        <span class="play-avatar" aria-hidden="true" />
        <Show
          when={props.handle}
          fallback={<span class="play-bar-name seat-empty">Waiting for a player…</span>}
        >
          {(handle) => (
            <span class="play-bar-name">
              {handle()}
              {props.you ? " (you)" : ""}
            </span>
          )}
        </Show>
        <span class="play-bar-seat">{props.seat}</span>
        <Show when={props.active}>
          <span class="play-bar-active" role="img" aria-label={`${props.seat} to move`} />
        </Show>
      </div>
      {props.children}
    </div>
  );
}
