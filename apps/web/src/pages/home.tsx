import { A } from "@solidjs/router";
import { For, Match, Show, Switch, createResource } from "solid-js";
import { gameApi } from "../api/game";
import type { CatalogEntry } from "../api/types";
import { CatalogCard } from "../components/catalog-card";
import Footer from "../components/Footer";
import Navbar from "../components/Navbar";
import { usePageTitle } from "../components/page-title";

/** The featured games come first; otherwise the catalogue's own order is kept. */
function featuredFirst(entries: CatalogEntry[]): CatalogEntry[] {
  return [...entries].sort((a, b) => Number(b.featured) - Number(a.featured));
}

interface Shelf {
  title: string;
  entries: CatalogEntry[];
}

const UNGROUPED_TITLE = "More board games";

/** One shelf per family, in the order the catalogue first mentions each, then one for the games
 *  that belong to no family. Each shelf puts its featured games first. */
export function shelves(entries: CatalogEntry[]): Shelf[] {
  const families = new Map<string, CatalogEntry[]>();
  const ungrouped: CatalogEntry[] = [];
  for (const entry of entries) {
    if (entry.family === null) {
      ungrouped.push(entry);
      continue;
    }
    families.set(entry.family, [...(families.get(entry.family) ?? []), entry]);
  }
  const grouped = [...families].map(([title, members]) => ({ title, entries: featuredFirst(members) }));
  if (ungrouped.length === 0) return grouped;
  return [...grouped, { title: UNGROUPED_TITLE, entries: featuredFirst(ungrouped) }];
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
  const grouped = () => props.entries.some((entry) => entry.family !== null);

  return (
    <Show
      when={props.entries.length > 0}
      fallback={<p class="feed-message">No games yet. Check back soon.</p>}
    >
      <Show
        when={grouped()}
        fallback={
          <div class="game-grid">
            <For each={featuredFirst(props.entries)}>{(entry) => <CatalogCard entry={entry} />}</For>
          </div>
        }
      >
        <For each={shelves(props.entries)}>{(shelf) => <GameShelf shelf={shelf} />}</For>
      </Show>
    </Show>
  );
}

function GameShelf(props: { shelf: Shelf }) {
  const count = () => props.shelf.entries.length;

  return (
    <section class="game-shelf" data-testid="game-shelf">
      <h3 class="shelf-title">
        {props.shelf.title}
        <span class="shelf-count">
          {count()} {count() === 1 ? "game" : "games"}
        </span>
      </h3>
      <div class="game-grid">
        <For each={props.shelf.entries}>
          {(entry) => <CatalogCard entry={entry} headingLevel={4} />}
        </For>
      </div>
    </section>
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
    </section>
  );
}
