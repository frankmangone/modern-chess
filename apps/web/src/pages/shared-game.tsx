import { useNavigate, useParams } from "@solidjs/router";
import { Match, Show, Switch, createEffect, createMemo, createResource, createSignal } from "solid-js";
import { gameApi } from "../api/game";
import { ApiError } from "../api/http";
import type { SharedGame } from "../api/types";
import { useAuth } from "../auth/session";
import { type SpecCheck, checkSpec } from "../builder/engine";
import { loadDrafts, newDraft, saveDrafts, uniqueName } from "../builder/drafts";
import { Playground } from "../builder/playground";
import { createRunGuard } from "../builder/run-guard";
import { readSkin } from "../builder/skin";
import { LOCAL } from "../builder/sync";
import Footer from "../components/Footer";
import Navbar from "../components/Navbar";
import { usePageTitle } from "../components/page-title";

/** Set while a shared game is being started and cleared once its board has drawn. A game someone else
 *  wrote runs in this tab, so one that hangs it must not be started again by reloading the page. */
const guard = createRunGuard("hubbable.shared-run");

type Opened = { kind: "game"; game: SharedGame } | { kind: "missing" } | { kind: "unreachable" };

async function open(secret: string): Promise<Opened> {
  try {
    return { kind: "game", game: await gameApi.getShared(secret) };
  } catch (error) {
    return error instanceof ApiError && error.status === 404 ? { kind: "missing" } : { kind: "unreachable" };
  }
}

/** A game shared by a private link: it plays here, in the visitor's own browser, and can be copied
 *  into the workbench to be changed. Nothing about who made it is known or shown. */
export function SharedGamePage() {
  const params = useParams<{ token: string }>();
  const navigate = useNavigate();
  const auth = useAuth();
  const [opened] = createResource(() => params.token, open);
  const game = () => {
    const now = opened();
    return now?.kind === "game" ? now.game : null;
  };
  usePageTitle(() => game()?.name ?? "A shared game");

  const [verdict] = createResource(game, (shared): Promise<SpecCheck> =>
    checkSpec(shared.rules).catch(() => ({
      ok: false as const,
      message: "The engine could not be loaded, so this game cannot be checked. Reload the page to try again.",
      line: null,
      column: null,
      path: null,
    })),
  );
  const skin = createMemo(() => readSkin(game()?.skin ?? ""));

  // A run that never finished drawing last time is not started again on its own.
  const [held, setHeld] = createSignal(guard.holds(params.token));
  const [started, setStarted] = createSignal(false);
  const playable = () => verdict()?.ok === true && skin().problem === null;
  const running = () => playable() && (started() || !held());
  function start() {
    setHeld(false);
    setStarted(true);
  }
  // From the moment the game is started until its board has drawn, a reload would start it again.
  createEffect(() => {
    if (running()) guard.set(params.token);
  });

  function copy() {
    const shared = game();
    if (!shared) return;
    const kept = loadDrafts();
    const copy = newDraft(
      uniqueName(shared.name, kept.drafts.map((d) => d.name)),
      shared.rules,
      shared.skin,
      auth.account() === null ? LOCAL : { kind: "dirty", version: null },
    );
    saveDrafts({ drafts: [...kept.drafts, copy], current: copy.id });
    navigate("/build");
  }

  return (
    <div class="page">
      <Navbar activeLink="build" />
      <main class="builder shared">
        <Switch>
          <Match when={opened.loading}>
            <p class="feed-message">Opening the game…</p>
          </Match>
          <Match when={opened()?.kind === "missing"}>
            <header class="builder-head">
              <h1 class="builder-title">This link does not open a game</h1>
              <p>The person who shared it may have stopped sharing, or the link was copied wrongly.</p>
              <p class="builder-links">
                <a href="/build" class="text-link">Open the workbench</a>
              </p>
            </header>
          </Match>
          <Match when={opened()?.kind === "unreachable"}>
            <header class="builder-head">
              <h1 class="builder-title">The game could not be opened</h1>
              <p>We could not reach the server. Please try again in a moment.</p>
            </header>
          </Match>
          <Match when={game()}>
            {(shared) => (
              <>
                <header class="builder-head">
                  <h1 class="builder-title">{shared().name}</h1>
                  <p>A game someone shared with you. It plays here, in your browser, against yourself.</p>
                </header>
                <section class="shared-play" aria-label="Play the game">
                  <Switch>
                    <Match when={verdict.loading}>
                      <p class="feed-message">Checking the game…</p>
                    </Match>
                    <Match when={verdict()?.ok === false ? (verdict() as Extract<SpecCheck, { ok: false }>) : null}>
                      {(problem) => (
                        <p class="builder-warning" role="alert">
                          The engine cannot read this game, so it cannot be played. {problem().message}
                        </p>
                      )}
                    </Match>
                    <Match when={skin().problem}>
                      {(problem) => (
                        <p class="builder-warning" role="alert">
                          The look of this game cannot be read, so it cannot be played. {problem()}
                        </p>
                      )}
                    </Match>
                    <Match when={held() && !started()}>
                      <p class="builder-warning" role="alert">
                        This game did not finish starting last time, so it was not started now.
                      </p>
                      <button type="button" class="btn-primary-lg" onClick={start}>
                        Start it anyway
                      </button>
                    </Match>
                    <Match when={running()}>
                      <Playground
                        rules={JSON.parse(shared().rules) as unknown}
                        skin={skin().skin}
                        onAlive={() => guard.set(null)}
                      />
                    </Match>
                  </Switch>
                </section>
                <div class="shared-actions">
                  <button type="button" class="btn-primary-lg" onClick={copy}>
                    Make my own copy
                  </button>
                  <a href="/build" class="text-link">Open the workbench</a>
                </div>
                <Show when={verdict()?.ok === true}>
                  <details class="builder-state shared-rules">
                    <summary>See the rules</summary>
                    <pre>{shared().rules}</pre>
                  </details>
                </Show>
              </>
            )}
          </Match>
        </Switch>
      </main>
      <Footer />
    </div>
  );
}
