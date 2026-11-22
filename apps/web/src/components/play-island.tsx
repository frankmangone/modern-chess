import { For, Show } from "solid-js";
import type { Answer, BoardView } from "../api/types";
import { PieceFace } from "./piece-face";

export interface PlayIslandProps {
  view: BoardView;
  skin: Record<string, unknown> | null;
  /** The viewer's seat, or null for someone only watching. */
  seat: string | null;
  /** True when the viewer may act right now. */
  interactive: boolean;
  /** Answers the open choice: a code, a hand piece or an action. */
  onChoose: (answer: Answer) => void;
  /** Takes back what was chosen so far. */
  onCancel: () => void;
}

/** "PASS" on a button reads "Pass". */
function actionLabel(name: string): string {
  const text = name.replaceAll("_", " ").toLowerCase();
  return text.charAt(0).toUpperCase() + text.slice(1);
}

/** The part of the play page that changes with the moment: the promotion choice, the hands and
 *  a line saying what to do next. */
export function PlayIsland(props: PlayIslandProps) {
  const choice = () => props.view.choice;
  const promoting = () => props.interactive && choice().codes.length > 0;
  /** The hand piece chosen so far this turn, if any. */
  const chosenHand = () => {
    for (const answer of choice().chosen) if ("hand" in answer) return answer.hand;
    return null;
  };
  /** The viewer's own hand comes first, and a hand with nothing in it is only worth showing to its owner. */
  const owners = () => {
    const { players, hand } = props.view;
    const worthShowing = (owner: string) => owner === props.seat || Object.keys(hand[owner] ?? {}).length > 0;
    const shown = players.filter(worthShowing);
    return props.seat === null ? shown : [...shown.filter((p) => p === props.seat), ...shown.filter((p) => p !== props.seat)];
  };
  /** The ring is for the player who has something to do; everyone else sees it quiet. */
  const yourTurn = () => props.seat !== null && props.view.turn === props.seat;
  const actions = () => (props.interactive && !promoting() ? choice().actions : []);
  const scores = () => Object.entries(props.view.score);
  const canPick = (owner: string) => owner === props.seat && props.interactive && !promoting();

  const hint = () => {
    const { view } = props;
    if (!props.interactive) return props.seat === null ? `${view.turn} to move.` : `Waiting for ${view.turn}…`;
    if (promoting()) return "Choose a promotion.";
    const hand = chosenHand();
    if (hand !== null) return `${hand} selected. Choose a highlighted square.`;
    if (choice().start !== null) return "Piece selected. Choose a highlighted square.";
    const narrowing = choice().actions.filter((action) => !action.plays);
    if (choice().chosen.length === 0 && narrowing.length > 0) {
      const aPiece = narrowing.length === 1 ? `a ${narrowing[0].name.toLowerCase()}` : "a piece";
      const passed = view.passes > 0 ? "The last turn was a pass. " : "";
      const canPass = choice().actions.some((action) => action.name === "PASS");
      return `${passed}Place ${aPiece} on an empty point${canPass ? ", or pass" : ""}.`;
    }
    if (choice().chosen.length > 0) return "Choose a highlighted square.";
    if (view.hands && Object.keys(view.hand[props.seat ?? ""] ?? {}).length > 0) {
      return "Move a piece, or drop one you hold.";
    }
    return "Select a piece to see where it can go.";
  };

  return (
    <section class="island" classList={{ "island-quiet": !yourTurn() }} aria-label="What to do next">
      <Show when={promoting()}>
        <header class="island-head">
          <span class="island-pulse" aria-hidden="true" />
          <span class="island-title">Action required</span>
        </header>
        <div class="island-choices">
          <For each={choice().codes}>
            {(code) => (
              <button
                type="button"
                class="island-choice"
                aria-label={`Promote to ${code}`}
                onClick={() => props.onChoose({ code })}
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
                              classList={{ "island-chip-chosen": chosenHand() === code }}
                              aria-label={`Drop ${code}`}
                              aria-pressed={chosenHand() === code}
                              disabled={chosenHand() !== code && !choice().hand.includes(code)}
                              onClick={() => (chosenHand() === code ? props.onCancel() : props.onChoose({ hand: code }))}
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

      <For each={actions()}>
        {(action) => (
          <button type="button" class="island-pass" onClick={() => props.onChoose({ action: action.name })}>
            {actionLabel(action.name)}
          </button>
        )}
      </For>
    </section>
  );
}
