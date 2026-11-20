import { Show, createEffect, createSignal, on } from "solid-js";

interface ResignControlProps {
  /** Something is already being sent, so a second request would only race the first. */
  busy: boolean;
  onResign: () => void;
}

/** "Resign", and the question that must be answered before it counts. */
export function ResignControl(props: ResignControlProps) {
  const [asking, setAsking] = createSignal(false);
  let ask: HTMLButtonElement | undefined;
  let keep: HTMLButtonElement | undefined;

  // Focus follows the question: it opens on the safe answer, and goes back to where it began.
  createEffect(
    on(
      asking,
      (now) => (now ? keep : ask)?.focus(),
      { defer: true },
    ),
  );

  function confirm() {
    setAsking(false);
    props.onResign();
  }

  return (
    <Show
      when={asking()}
      fallback={
        <button ref={ask} type="button" class="play-resign" disabled={props.busy} onClick={() => setAsking(true)}>
          Resign
        </button>
      }
    >
      <div
        class="play-resign-prompt"
        role="group"
        aria-labelledby="resign-question"
        onKeyDown={(event) => {
          if (event.key === "Escape") setAsking(false);
        }}
      >
        <span id="resign-question" class="play-resign-question">
          Resign this match?
        </span>
        <button type="button" class="play-resign-confirm" disabled={props.busy} onClick={confirm}>
          Yes, resign
        </button>
        <button ref={keep} type="button" class="play-resign-keep" onClick={() => setAsking(false)}>
          Keep playing
        </button>
      </div>
    </Show>
  );
}
