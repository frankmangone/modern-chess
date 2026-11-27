import { A } from "@solidjs/router";
import { For, Match, Show, Switch, createMemo, createResource, createSignal } from "solid-js";
import { gameApi } from "../api/game";
import type { CatalogEntry } from "../api/types";
import { CatalogCard, genreTags } from "../components/catalog-card";
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

/** How many games the featured band holds: one large, and two beside it. */
export const BAND_SIZE = 3;

/** The band: the featured games, in the order the front page lists them, up to its size. A game
 *  nobody pinned is not in it, so with nothing featured there is no band. */
export function band(items: Listing[]): Listing[] {
  return items.filter((item) => item.game.featured).slice(0, BAND_SIZE);
}

/** The kinds of game to filter by, each once, in the order they first appear. */
export function kinds(items: Listing[]): string[] {
  return [...new Set(items.flatMap((item) => genreTags(item.game)))];
}

const titled = (tag: string) => `${tag[0]?.toUpperCase() ?? ""}${tag.slice(1)} games`;

export function HomePage() {
  const [catalog] = createResource(() => gameApi.getCatalog());
  usePageTitle(() => undefined);

  return (
    <div class="page">
      <Navbar activeLink="games" />
      <main>
        <Hero count={catalog.state === "ready" ? catalog().length : undefined} />
        <section class="game-feed" id="games" aria-labelledby="games-title">
          <Switch>
            <Match when={catalog.state === "errored"}>
              <FeedTitle>Play one today</FeedTitle>
              <p class="feed-message" role="alert">
                We can't reach the game server right now. Please try again in a moment.
              </p>
            </Match>
            <Match when={catalog.state === "ready"}>
              <Feed entries={catalog() ?? []} />
            </Match>
            <Match when={true}>
              <FeedTitle>Play one today</FeedTitle>
              <p class="feed-message">Loading games…</p>
            </Match>
          </Switch>
        </section>
      </main>
      <Footer />
    </div>
  );
}

function FeedTitle(props: { children: string }) {
  return (
    <h2 id="games-title" class="feed-title">
      {props.children}
    </h2>
  );
}

function Feed(props: { entries: CatalogEntry[] }) {
  const items = createMemo(() => listings(props.entries));
  const featured = createMemo(() => band(items()));
  const [kind, setKind] = createSignal<string | null>(null);
  // The band stands for "everything"; once a kind is picked, its games are all in the list.
  const rest = () =>
    kind() === null
      ? items().filter((item) => !featured().includes(item))
      : items().filter((item) => item.game.tags.includes(kind() as string));
  const title = () => (kind() !== null ? titled(kind() as string) : featured().length > 0 ? "More to play" : "Play one today");

  return (
    <Show
      when={props.entries.length > 0}
      fallback={
        <>
          <FeedTitle>Play one today</FeedTitle>
          <p class="feed-message">No games yet. Check back soon.</p>
        </>
      }
    >
      <Show when={kind() === null && featured().length > 0}>
        <div class="feed-section">
          <div class="feed-head">
            <h2 class="feed-title" id="featured-title">
              Featured
            </h2>
          </div>
          <div class="game-band" classList={{ "is-single": featured().length === 1 }}>
            <For each={featured().slice(0, 1)}>
              {(item) => <CatalogCard entry={item.game} variantCount={item.variants.length} size="lead" />}
            </For>
            <Show when={featured().length > 1}>
              <div class="game-band-stack">
                <For each={featured().slice(1)}>
                  {(item) => <CatalogCard entry={item.game} variantCount={item.variants.length} size="side" />}
                </For>
              </div>
            </Show>
          </div>
        </div>
      </Show>
      <div class="feed-section">
        <div class="feed-head">
          <FeedTitle>{title()}</FeedTitle>
          <div class="feed-filters" role="group" aria-label="Filter by kind of game">
            <button class="feed-filter" aria-pressed={kind() === null} onClick={() => setKind(null)}>
              All
            </button>
            <For each={kinds(items())}>
              {(tag) => (
                <button class="feed-filter" aria-pressed={kind() === tag} onClick={() => setKind(tag)}>
                  {tag}
                </button>
              )}
            </For>
          </div>
        </div>
        <Show when={rest().length > 0}>
          <div class="game-grid">
            <For each={rest()}>
              {(item) => <CatalogCard entry={item.game} variantCount={item.variants.length} />}
            </For>
          </div>
        </Show>
      </div>
    </Show>
  );
}

function Hero(props: { count: number | undefined }) {
  return (
    <section class="hero" aria-labelledby="hero-title">
      <h1 id="hero-title" class="hero-title">
        Write the rules. Play the game.
      </h1>

      <div class="hero-aside">
        <p class="hero-sub">
          Every game here is a page of rules, played by one engine: Chess and Go, Backgammon and
          Kalah, and the odd ones you have never heard of. Pick one and send a friend the link, no
          sign-up needed. Or write a game of your own.
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
          <A href="/build" class="text-link">
            Write your own
          </A>
        </div>
      </div>

      <HeroBoard />
    </section>
  );
}
