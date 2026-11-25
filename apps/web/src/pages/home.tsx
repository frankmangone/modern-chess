import { A } from "@solidjs/router";
import { For, Match, Show, Switch, createResource } from "solid-js";
import { gameApi } from "../api/game";
import type { CatalogEntry } from "../api/types";
import { CatalogCard } from "../components/catalog-card";
import Footer from "../components/Footer";
import { HeroBoard } from "../components/hero-board";
import Navbar from "../components/Navbar";
import { usePageTitle } from "../components/page-title";

/** The featured games come first; otherwise the catalogue's own order is kept. */
function featuredFirst(entries: CatalogEntry[]): CatalogEntry[] {
  return [...entries].sort((a, b) => Number(b.featured) - Number(a.featured));
}

export interface Listing {
  game: CatalogEntry;
  /** The games that are variants of it, in the catalogue's order. */
  variants: CatalogEntry[];
}

/** The games the front page shows a card for, each with its variants. A variant goes under its
 *  parent; one whose parent is not listed stands on its own, so no game ever goes missing. */
export function listings(entries: CatalogEntry[]): Listing[] {
  const listed = new Set(entries.map((entry) => entry.slug));
  const isVariant = (entry: CatalogEntry) => entry.parent !== null && listed.has(entry.parent);
  return featuredFirst(entries.filter((entry) => !isVariant(entry))).map((game) => ({
    game,
    variants: entries.filter((entry) => isVariant(entry) && entry.parent === game.slug),
  }));
}

export function HomePage() {
  const [catalog] = createResource(() => gameApi.getCatalog());
  usePageTitle(() => undefined);

  return (
    <div class="page">
      <Navbar activeLink="games" />
      <main>
        <Hero count={catalog.state === "ready" ? catalog().length : undefined} />
        <section class="game-feed" id="games" aria-labelledby="games-title">
          <h2 id="games-title" class="feed-title">
            Play one today
          </h2>
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
      </main>
      <Footer />
    </div>
  );
}

function Feed(props: { entries: CatalogEntry[] }) {
  return (
    <Show
      when={props.entries.length > 0}
      fallback={<p class="feed-message">No games yet. Check back soon.</p>}
    >
      <div class="game-grid">
        <For each={listings(props.entries)}>
          {(listing) => <CatalogCard entry={listing.game} variantGames={listing.variants} />}
        </For>
      </div>
    </Show>
  );
}

function Hero(props: { count: number | undefined }) {
  return (
    <section class="hero" aria-labelledby="hero-title">
      <h1 id="hero-title" class="hero-title">
        Every board game, in one place.
      </h1>

      <div class="hero-aside">
        <p class="hero-sub">
          Pick a game, open a lobby, and play a friend. No sign-up: a guest name is made for you
          the first time you play.
        </p>
        <Show when={props.count}>
          <p class="hero-count">
            {props.count} {props.count === 1 ? "game" : "games"} available now
          </p>
        </Show>
        <div class="hero-ctas">
          <A href="#games" class="btn-primary-lg">
            Browse the games
          </A>
        </div>
      </div>

      <HeroBoard />
    </section>
  );
}
