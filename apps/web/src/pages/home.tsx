import { A } from "@solidjs/router";
import { For, Match, Show, Switch, createResource } from "solid-js";
import { gameApi } from "../api/game";
import type { CatalogEntry } from "../api/types";
import { CatalogCard } from "../components/catalog-card";
import Footer from "../components/Footer";
import Navbar from "../components/Navbar";

const SMALL_CARDS = 3;

interface Arrangement {
  featured: CatalogEntry;
  small: CatalogEntry[];
  wide: CatalogEntry[];
}

/** The featured game gets the big card; the next few the side column; the rest a row below. */
function arrange(entries: CatalogEntry[]): Arrangement | null {
  const featured = entries.find((e) => e.featured) ?? entries[0];
  if (featured === undefined) return null;
  const rest = entries.filter((e) => e !== featured);
  return { featured, small: rest.slice(0, SMALL_CARDS), wide: rest.slice(SMALL_CARDS) };
}

export function HomePage() {
  const [catalog] = createResource(() => gameApi.getCatalog());

  return (
    <div>
      <Navbar activeLink="games" />
      <Hero count={catalog.state === "ready" ? catalog().length : undefined} games={catalog.state === "ready" ? catalog() : []} />
      <ChessStrip />
      <section class="game-feed">
        <div class="feed-header">
          <div class="feed-title-group">
            <span class="feed-eyebrow">✦ DISCOVER ✦</span>
            <h2 class="feed-title">Explore Games</h2>
          </div>
        </div>
        <Switch>
          <Match when={catalog.state === "errored"}>
            <p class="feed-message" role="alert">
              We can't reach the game server right now. Please try again in a moment.
            </p>
          </Match>
          <Match when={catalog.state === "ready"}>
            <Feed entries={catalog() ?? []} />
          </Match>
          <Match when={true}>
            <p class="feed-message">Loading games…</p>
          </Match>
        </Switch>
      </section>
      <Footer />
    </div>
  );
}

function Feed(props: { entries: CatalogEntry[] }) {
  const layout = () => arrange(props.entries);

  return (
    <Show when={layout()} fallback={<p class="feed-message">No games yet. Check back soon.</p>}>
      {(arranged) => (
        <>
          <div class="game-grid">
            <CatalogCard entry={arranged().featured} variant="featured" />
            <div class="small-cards-col">
              <For each={arranged().small}>
                {(entry) => <CatalogCard entry={entry} variant="small" />}
              </For>
            </div>
          </div>
          <Show when={arranged().wide.length > 0}>
            <div class="row2-cards">
              <For each={arranged().wide}>
                {(entry) => <CatalogCard entry={entry} variant="wide" />}
              </For>
            </div>
          </Show>
        </>
      )}
    </Show>
  );
}

function Hero(props: { count: number | undefined; games: CatalogEntry[] }) {
  return (
    <section class="hero">
      <Show when={props.count}>
        <div class="hero-badge">
          <div class="hero-badge-dot" />
          <span class="hero-badge-text">
            ✨ {props.count} {props.count === 1 ? "game" : "games"} available now
          </span>
        </div>
      </Show>

      <h1 class="hero-title">
        The Hub for{"\n"}Every Board Game
      </h1>

      <p class="hero-sub">
        Pick a game, open a lobby, and play a friend. No sign-up: a guest name is made for you
        the first time you play.
      </p>

      <div class="hero-ctas">
        <A href="#games" class="btn-primary-lg">Explore Games</A>
      </div>

      <Show when={props.games.length > 0}>
        <div class="hero-pills">
          <For each={props.games.slice(0, 4)}>
            {(game) => (
              <A href={`/games/${game.slug}`} class="hero-pill" style={{ color: game.accent ?? undefined }}>
                {game.name}
              </A>
            )}
          </For>
          <Show when={props.games.length > 4}>
            <span class="hero-pill muted">+ more</span>
          </Show>
        </div>
      </Show>
    </section>
  );
}

function ChessStrip() {
  return (
    <div class="chess-strip">
      <span class="chess-strip-text">
        ♛  ♜  ♞  ♝  ♟  ·  ♛  ♜  ♞  ♝  ♟  ·  ♛  ♜  ♞  ♝  ♟  ·  ♛  ♜  ♞  ♝  ♟  ·  ♛  ♜  ♞  ♝  ♟  ·  ♛  ♜  ♞  ♝  ♟
      </span>
    </div>
  );
}
