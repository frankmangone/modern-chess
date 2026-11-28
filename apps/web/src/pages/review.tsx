import { For, Match, Show, Switch, createResource, createSignal } from "solid-js";
import { gameApi } from "../api/game";
import { ApiError } from "../api/http";
import type { ReviewLine } from "../api/types";
import { withAccount } from "../auth/identity";
import { useAuth } from "../auth/session";
import { StrangerGame } from "../builder/stranger-game";
import Footer from "../components/Footer";
import Navbar from "../components/Navbar";
import { usePageTitle } from "../components/page-title";

/** Set while a game under review is being started and cleared once its board has drawn. */
const GUARD_KEY = "hubbable.review-run";

type Queue = { kind: "games"; games: ReviewLine[] } | { kind: "not-staff" } | { kind: "unreachable" };

async function loadQueue(): Promise<Queue> {
  try {
    return { kind: "games", games: await withAccount((token) => gameApi.reviewQueue(token)) };
  } catch (error) {
    return error instanceof ApiError && error.status === 403 ? { kind: "not-staff" } : { kind: "unreachable" };
  }
}

function when(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? "" : date.toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
}

/** For the people who look at a new author's first game before it appears on the hub: the games
 *  waiting, each one playable here, and a way to let it through or send it back with a reason. */
export function ReviewPage() {
  usePageTitle(() => "Review");
  const auth = useAuth();
  const signedIn = () => auth.account() !== null;
  const [queue, { refetch }] = createResource(() => signedIn() && auth.ready(), (ready) => (ready ? loadQueue() : undefined));
  const [selected, setSelected] = createSignal<string | null>(null);
  const [game] = createResource(selected, (id) => withAccount((token) => gameApi.reviewGame(id, token)));
  const [reason, setReason] = createSignal("");
  const [sendingBack, setSendingBack] = createSignal(false);
  const [working, setWorking] = createSignal(false);
  const [message, setMessage] = createSignal<string | null>(null);
  const [problem, setProblem] = createSignal<string | null>(null);
  const [playable, setPlayable] = createSignal<boolean | null>(null);

  const games = () => {
    const now = queue();
    return now?.kind === "games" ? now.games : [];
  };

  function pick(id: string) {
    setSelected(id);
    setSendingBack(false);
    setReason("");
    setProblem(null);
    setPlayable(null);
  }

  async function settle(done: () => Promise<string>) {
    setWorking(true);
    setProblem(null);
    try {
      setMessage(await done());
      setSelected(null);
      await refetch();
    } catch (error) {
      setProblem(error instanceof ApiError ? error.message : "We could not reach the server. Please try again in a moment.");
    } finally {
      setWorking(false);
    }
  }
  const approve = (id: string, name: string) =>
    settle(async () => {
      await withAccount((token) => gameApi.approve(id, token));
      return `“${name}” is on the hub.`;
    });
  const sendBack = (id: string, name: string) =>
    settle(async () => {
      await withAccount((token) => gameApi.reject(id, reason(), token));
      return `“${name}” went back to its author.`;
    });

  return (
    <div class="page">
      <Navbar activeLink="build" />
      <main class="builder">
        <header class="builder-head">
          <h1 class="builder-title">Review</h1>
          <p>The first game of each new author is looked at before it appears on the hub. Play it, read what its author wrote, and let it through or send it back.</p>
        </header>

        <Switch>
          <Match when={!auth.ready() || queue.loading}>
            <p class="feed-message">Loading…</p>
          </Match>
          <Match when={!signedIn()}>
            <p class="feed-message">
              <a class="text-link" href="/sign-in?next=%2Freview">Sign in</a> to see the games waiting.
            </p>
          </Match>
          <Match when={queue()?.kind === "not-staff"}>
            <p class="feed-message">This page is for the people who look at new games.</p>
          </Match>
          <Match when={queue()?.kind === "unreachable"}>
            <p class="feed-message">We could not reach the server. Please try again in a moment.</p>
          </Match>
          <Match when={true}>
            <Show when={message()}>{(text) => <p class="builder-done" role="status"><span>{text()}</span></p>}</Show>
            <Show when={games().length > 0} fallback={<p class="feed-message">Nothing is waiting.</p>}>
              <div class="review-grid">
                <ul class="review-queue" aria-label="Games waiting">
                  <For each={games()}>
                    {(line) => (
                      <li>
                        <button type="button" class="review-item" aria-current={selected() === line.id} onClick={() => pick(line.id)}>
                          <span class="review-item-name">{line.name}</span>
                          <span class="review-item-meta">by {line.author}, {when(line.submitted_at)}</span>
                        </button>
                      </li>
                    )}
                  </For>
                </ul>

                <div class="review-detail">
                  <Show when={selected()} fallback={<p class="feed-message">Pick a game to read it and play it.</p>}>
                    <Switch>
                      <Match when={game.loading}>
                        <p class="feed-message">Opening the game…</p>
                      </Match>
                      <Match when={game.error}>
                        <p class="builder-warning" role="alert">This game could not be opened. It may have been taken back, or looked at by someone else.</p>
                      </Match>
                      <Match when={game()}>
                        {(open) => (
                          <>
                            <h2 class="builder-title">{open().name}</h2>
                            <div class="review-words">
                              <p class="review-tagline">{open().listing?.tagline}</p>
                              <p>{open().listing?.description}</p>
                              <p class="builder-share-note">By {open().author}, under {open().listing?.licence}.</p>
                            </div>
                            <Show when={open()} keyed>
                              {(shown) => (
                                <StrangerGame
                                  rules={shown.rules}
                                  skin={shown.skin}
                                  id={shown.id}
                                  guardKey={GUARD_KEY}
                                  onChecked={setPlayable}
                                />
                              )}
                            </Show>
                            <details class="builder-state shared-rules">
                              <summary>See the rules</summary>
                              <pre>{open().rules}</pre>
                            </details>
                            <Show when={problem()}>{(text) => <p class="builder-warning" role="alert">{text()}</p>}</Show>
                            <Show
                              when={sendingBack()}
                              fallback={
                                <div class="review-actions">
                                  <button type="button" class="btn-primary-lg" disabled={working() || playable() === false} onClick={() => void approve(open().id, open().name)}>
                                    Let it through
                                  </button>
                                  <button type="button" class="builder-small" disabled={working()} onClick={() => setSendingBack(true)}>
                                    Send it back…
                                  </button>
                                  <Show when={playable() === false}>
                                    <span class="builder-hint">The engine cannot read it, so it cannot be let through.</span>
                                  </Show>
                                </div>
                              }
                            >
                              <label class="review-reason">
                                Why it goes back (its author reads this)
                                <textarea value={reason()} maxLength={1000} onInput={(event) => setReason(event.currentTarget.value)} />
                              </label>
                              <div class="review-actions">
                                <button type="button" class="btn-primary-lg" disabled={working() || reason().trim() === ""} onClick={() => void sendBack(open().id, open().name)}>
                                  Send it back
                                </button>
                                <button type="button" class="builder-small" disabled={working()} onClick={() => setSendingBack(false)}>
                                  Cancel
                                </button>
                              </div>
                            </Show>
                          </>
                        )}
                      </Match>
                    </Switch>
                  </Show>
                </div>
              </div>
            </Show>
          </Match>
        </Switch>
      </main>
      <Footer />
    </div>
  );
}
