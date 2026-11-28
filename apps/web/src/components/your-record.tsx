import { For, Show, createResource } from "solid-js";
import { gameApi } from "../api/game";
import { loadGuest } from "../auth/token-store";
import { RecordTag, recordSentence } from "./record-tag";

/** What the visitor has done on this game, with a line for each variant they have finished a match
 *  in. Not shown to someone with no finished match, no remembered name, or when it cannot be read:
 *  it is a nicety, and never worth an error on the page. */
export function YourRecord(props: { game: string }) {
  const [record] = createResource(
    () => ({ game: props.game, token: loadGuest()?.token ?? null }),
    async ({ game, token }) => {
      if (token === null) return null;
      try {
        return await gameApi.getMyRecord(game, token);
      } catch {
        return null;
      }
    },
  );
  const shown = () => {
    const mine = record();
    return mine && mine.total.won + mine.total.drawn + mine.total.lost > 0 ? mine : null;
  };

  return (
    <Show when={shown()}>
      {(mine) => (
        <section class="your-record" aria-label="Your record on this game">
          <h3>Your record</h3>
          <p class="your-record-total">
            <RecordTag record={mine().total} />
            <span class="your-record-sentence">{recordSentence(mine().total)}</span>
          </p>
          <Show when={mine().variants.length > 1}>
            <ul class="your-record-variants">
              <For each={mine().variants}>
                {(variant) => (
                  <li>
                    <span>{variant.label}</span>
                    <RecordTag record={variant.record} />
                  </li>
                )}
              </For>
            </ul>
          </Show>
        </section>
      )}
    </Show>
  );
}
