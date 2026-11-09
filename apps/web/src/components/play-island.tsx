import { normalizeSkin, pieceFace } from "@board/board-model.js";
import { For, Match, Show, Switch } from "solid-js";
import type { BoardView } from "../api/types";

interface PieceFaceProps {
  skin: Record<string, unknown> | null;
  code: string;
  player: string;
  players: string[];
}

/** A piece drawn the way the board draws it, so a hand matches the game's skin. */
function PieceFace(props: PieceFaceProps) {
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

export interface PlayIslandProps {
  view: BoardView;
  skin: Record<string, unknown> | null;
  /** The viewer's seat, or null for someone only watching. */
  seat: string | null;
  /** True when the viewer may act right now. */
  interactive: boolean;
  /** The piece picked from the hand, or null. */
  dropCode: string | null;
  onPickDrop: (code: string | null) => void;
  onPromote: (code: string) => void;
}

/** The part of the play page that changes with the moment: the promotion choice, the hands and
 *  a line saying what to do next. */
export function PlayIsland(props: PlayIslandProps) {
  const promoting = () => props.interactive && props.view.promotion !== null;
  /** The viewer's own hand comes first. */
  const owners = () => {
    const { players } = props.view;
    return props.seat === null ? players : [...players.filter((p) => p === props.seat), ...players.filter((p) => p !== props.seat)];
  };
  const canPick = (owner: string) => owner === props.seat && props.interactive && !promoting();

  const hint = () => {
    const { view } = props;
    if (!props.interactive) return props.seat === null ? `${view.turn} to move.` : `Waiting for ${view.turn}…`;
    if (promoting()) return "Choose a promotion.";
    if (props.dropCode !== null) return `${props.dropCode} selected. Choose a highlighted square.`;
    if (view.selected !== null) return "Piece selected. Choose a highlighted square.";
    if (view.hands && Object.keys(view.hand[props.seat ?? ""] ?? {}).length > 0) {
      return "Move a piece, or drop one you hold.";
    }
    return "Select a piece to see where it can go.";
  };

  return (
    <section class="island" aria-label="What to do next">
      <Show when={promoting()}>
        <header class="island-head">
          <span class="island-pulse" aria-hidden="true" />
          <span class="island-title">ACTION REQUIRED</span>
        </header>
        <div class="island-choices">
          <For each={props.view.promotion ?? []}>
            {(code) => (
              <button
                type="button"
                class="island-choice"
                aria-label={`Promote to ${code}`}
                onClick={() => props.onPromote(code)}
              >
                <PieceFace skin={props.skin} code={code} player={props.view.turn} players={props.view.players} />
                <span class="island-choice-name">{code}</span>
              </button>
            )}
          </For>
        </div>
      </Show>

      <Show when={props.view.hands}>
        <For each={owners()}>
          {(owner) => {
            const held = () => Object.entries(props.view.hand[owner] ?? {});
            return (
              <div class="island-hand">
                <p class="island-hand-label">{owner === props.seat ? "Your hand" : `${owner}'s hand`}</p>
                <Show when={held().length > 0} fallback={<p class="island-hand-empty">Nothing in hand</p>}>
                  <div class="island-chips">
                    <For each={held()}>
                      {([code, count]) => {
                        const face = () => (
                          <>
                            <PieceFace skin={props.skin} code={code} player={owner} players={props.view.players} />
                            <span class="island-chip-label">
                              {code} ×{count}
                            </span>
                          </>
                        );
                        return (
                          <Show
                            when={canPick(owner)}
                            fallback={<span class="island-chip island-chip-static">{face()}</span>}
                          >
                            <button
                              type="button"
                              class="island-chip"
                              classList={{ "island-chip-chosen": props.dropCode === code }}
                              aria-label={`Drop ${code}`}
                              aria-pressed={props.dropCode === code}
                              disabled={(props.view.drops[code] ?? []).length === 0}
                              onClick={() => props.onPickDrop(props.dropCode === code ? null : code)}
                            >
                              {face()}
                            </button>
                          </Show>
                        );
                      }}
                    </For>
                  </div>
                </Show>
              </div>
            );
          }}
        </For>
      </Show>

      <p class="island-hint">{hint()}</p>
    </section>
  );
}
