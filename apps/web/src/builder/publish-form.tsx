import { Show, createSignal } from "solid-js";
import type { Listing } from "../api/types";
import { LICENCE, LICENCE_NAME, LICENCE_URL, MAX_DESCRIPTION, MAX_TAGLINE, type Publishing, listingProblem } from "./publishing";

interface Props {
  /** What the game is called now. */
  name: string;
  /** The words it already has, when it has been handed in before or is a revision of a game on the hub. */
  initial: Listing | null;
  /** Whether this is a new revision of a game that is already on the hub. */
  revision: boolean;
  /** Whether the game will wait for staff: true for an author's first game. */
  review: boolean;
  state: Publishing;
  onSend: (listing: Listing) => void;
  onCancel: () => void;
}

/** What a person says about a game when they hand it in: a line, a description, and the licence they
 *  agree to. The server has the last word on all of it; this only keeps the obvious mistakes from
 *  being sent. */
export function PublishForm(props: Props) {
  const [tagline, setTagline] = createSignal(props.initial?.tagline ?? "");
  const [description, setDescription] = createSignal(props.initial?.description ?? "");
  const [accepted, setAccepted] = createSignal(false);
  const [tried, setTried] = createSignal(false);

  const listing = (): Listing => ({ tagline: tagline(), description: description(), licence: LICENCE });
  const problem = () => listingProblem(listing(), accepted());
  const sending = () => props.state.kind === "sending";
  const verb = () => (props.review && !props.revision ? "Hand in for review" : "Publish");

  function submit(event: SubmitEvent) {
    event.preventDefault();
    setTried(true);
    if (problem() === null) props.onSend(listing());
  }

  return (
    <form class="builder-publish" onSubmit={submit} aria-label="Publish this game">
      <h2 class="builder-publish-title">{props.revision ? `A new revision of “${props.name}”` : `Publish “${props.name}”`}</h2>
      <p class="builder-publish-lede">
        <Show
          when={props.revision}
          fallback={
            props.review
              ? "This is your first game, so we look at it before it appears on the hub. You can take it back until then."
              : "It goes on the hub as soon as you publish it, and anyone can open a lobby in it."
          }
        >
          It replaces what is on the hub. Matches already started keep the rules they began with.
        </Show>
      </p>

      <label class="builder-publish-field">
        <span>Tagline</span>
        <input
          type="text"
          value={tagline()}
          maxLength={MAX_TAGLINE}
          placeholder="Two armies, one road."
          disabled={sending()}
          onInput={(event) => setTagline(event.currentTarget.value)}
        />
      </label>
      <label class="builder-publish-field">
        <span>Description</span>
        <textarea
          rows={5}
          value={description()}
          maxLength={MAX_DESCRIPTION}
          placeholder="What the game is, and what makes it worth playing."
          disabled={sending()}
          onInput={(event) => setDescription(event.currentTarget.value)}
        />
      </label>

      <label class="builder-publish-licence">
        <input type="checkbox" checked={accepted()} disabled={sending()} onChange={(event) => setAccepted(event.currentTarget.checked)} />
        <span>
          Share this game under {LICENCE_NAME}.{" "}
          <span class="builder-publish-hint">
            Anyone can play it, copy it and build on it, as long as they credit you and share their changes the same way.{" "}
            <a class="text-link" href={LICENCE_URL} target="_blank" rel="noreferrer">Read the licence</a>.
          </span>
        </span>
      </label>

      <Show when={props.state.kind === "refused" ? props.state.reason : null}>
        {(reason) => <p class="builder-warning" role="alert">{reason()}</p>}
      </Show>
      <Show when={tried() && problem()}>{(text) => <p class="builder-warning" role="alert">{text()}</p>}</Show>

      <div class="builder-publish-actions">
        <button type="submit" class="btn-primary-lg" disabled={sending()}>
          {sending() ? "Sending…" : verb()}
        </button>
        <button type="button" class="builder-small" disabled={sending()} onClick={props.onCancel}>
          Cancel
        </button>
      </div>
    </form>
  );
}
