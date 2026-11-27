import { A, useParams } from "@solidjs/router";
import { Match, Show, Switch, createResource } from "solid-js";
import { ApiError } from "../api/http";
import { gameApi } from "../api/game";
import Footer from "../components/Footer";
import { LiveBoard } from "../components/live-board";
import { Markdown } from "../components/markdown";
import Navbar from "../components/Navbar";
import { usePageTitle } from "../components/page-title";
import { hasSection, howToPlay } from "../components/rules-text";
import { useTheme } from "../theme";

/** A game's rules, on a page of their own, with its board beside them to try what they say. */
export function RulesPage() {
  const params = useParams<{ slug: string }>();
  const [game] = createResource(
    () => params.slug,
    (slug) => gameApi.getGame(slug),
  );
  usePageTitle(() => (game.state === "ready" ? `How to play ${game()?.page.name}` : undefined));
  useTheme(() => (game.state === "ready" ? game()?.page.theme : null));

  return (
    <div class="page">
      <Navbar activeLink="games" />
      <main>
        <Switch>
          <Match when={game.state === "errored"}>
            <p class="feed-message" role="alert">
              {game.error instanceof ApiError && game.error.status === 404
                ? "We couldn't find this game."
                : "We can't reach the game server right now. Please try again in a moment."}
            </p>
          </Match>
          <Match when={game()}>
            {(detail) => (
              <section class="rules-page">
                <nav class="breadcrumb" aria-label="Breadcrumb">
                  <A href="/" class="breadcrumb-parent">Games</A>
                  <span class="breadcrumb-sep">›</span>
                  <A href={`/games/${detail().page.slug}`} class="breadcrumb-parent">
                    {detail().page.name}
                  </A>
                  <span class="breadcrumb-sep">›</span>
                  <span class="breadcrumb-current">How to play</span>
                </nav>
                <div class="rules-layout">
                  <div class="rules-text">
                    <h1 class="game-page-title">How to play {detail().page.name}</h1>
                    <Show
                      when={howToPlay(detail())}
                      fallback={<p class="feed-message">The rules of this game have not been written yet.</p>}
                    >
                      {(text) => <Markdown source={text()} />}
                    </Show>
                    <div class="game-actions">
                      <A href={`/games/${detail().page.slug}#play`} class="btn-primary-lg">
                        Play a friend
                      </A>
                      <A href={`/games/${detail().page.slug}`} class="text-link">
                        Back to {detail().page.name}
                      </A>
                    </div>
                  </div>
                  <Show when={hasSection(detail(), "preview")}>
                    <div class="game-hero-aside">
                      <LiveBoard detail={detail()} />
                      <p class="rules-note">This board is live. Try what the rules say.</p>
                    </div>
                  </Show>
                </div>
              </section>
            )}
          </Match>
          <Match when={true}>
            <p class="feed-message">Loading the rules…</p>
          </Match>
        </Switch>
      </main>
      <Footer />
    </div>
  );
}
