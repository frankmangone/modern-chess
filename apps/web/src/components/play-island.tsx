import { For, Show } from "solid-js";
import type { BoardView } from "../api/types";
import { PieceFace } from "./piece-face";

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
  onPass: () => void;
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
  const canPass = () => props.interactive && props.view.can_pass && !promoting();
  const scores = () => Object.entries(props.view.score);
  const canPick = (owner: string) => owner === props.seat && props.interactive && !promoting();

  const hint = () => {
    const { view } = props;
    if (!props.interactive) return props.seat === null ? `${view.turn} to move.` : `Waiting for ${view.turn}…`;
    if (promoting()) return "Choose a promotion.";
    if (props.dropCode !== null) return `${props.dropCode} selected. Choose a highlighted square.`;
    if (view.selected !== null) return "Piece selected. Choose a highlighted square.";
    if (view.supply.length > 0) {
      const aPiece = view.supply.length === 1 ? `a ${view.supply[0].toLowerCase()}` : "a piece";
      const passed = view.passes > 0 ? "The last turn was a pass. " : "";
      return `${passed}Place ${aPiece} on an empty point${view.can_pass ? ", or pass" : ""}.`;
    }
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

      <Show when={scores().length > 0}>
        <div class="island-score">
          <p class="island-hand-label">Count if the game ended now</p>
          <For each={scores()}>
            {([player, points]) => (
              <p class="island-score-row">
                <span>{player}</span>
                <span class="island-score-points">{points}</span>
              </p>
            )}
          </For>
        </div>
      </Show>

      <p class="island-hint">{hint()}</p>

      <Show when={canPass()}>
        <button type="button" class="island-pass" onClick={() => props.onPass()}>
          Pass
        </button>
      </Show>
    </section>
  );
}
