import { A, useLocation, useNavigate, useParams } from "@solidjs/router";
import {
  For,
  Match,
  Show,
  Switch,
  createEffect,
  createResource,
  createSignal,
  on,
} from "solid-js";
import { ApiError } from "../api/http";
import { gameApi } from "../api/game";
import type { GameDetail } from "../api/types";
import { withGuest } from "../auth/identity";
import { loadGuest } from "../auth/token-store";
import { CatalogCard } from "../components/catalog-card";
import Footer from "../components/Footer";
import { GameBoard } from "../components/game-board";
import type { LobbyActions } from "../components/lobby-list";
import Navbar from "../components/Navbar";
import { usePageTitle } from "../components/page-title";
import { SectionRenderer } from "../components/section-renderer";

const FALLBACK_ACCENT = "#8B4513";

/** A sentence a player can read. A 4xx message is written for that. A 5xx is the server, or
 *  the proxy in front of it, failing, and its text is not meant for a player. Anything else is
 *  the server being out of reach. */
function describe(error: unknown): string {
  if (error instanceof ApiError && error.status >= 500) {
    return "Something went wrong on our side. Please try again in a moment.";
  }
  if (error instanceof ApiError) return error.message;
  return "We can't reach the game server right now. Please try again in a moment.";
}

export function GameDetailPage() {
  const params = useParams<{ slug: string }>();
  const navigate = useNavigate();
  const [game] = createResource(
    () => params.slug,
    (slug) => gameApi.getGame(slug),
  );
  usePageTitle(() => (game.state === "ready" ? game()?.page.name : undefined));
  const location = useLocation();
  // A link to a part of the page (a card's "7 variants") arrives before the page has loaded, so
  // the browser finds nothing to scroll to: go there once the game is shown.
  createEffect(
    on([() => game.state, () => location.hash], ([state, hash]) => {
      if (state !== "ready" || !hash) return;
      queueMicrotask(() => document.getElementById(hash.slice(1))?.scrollIntoView());
    }),
  );
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  async function enter(open: (token: string) => Promise<string>) {
    setBusy(true);
    setError(null);
    try {
      navigate(`/play/${await withGuest(open)}`);
    } catch (failure) {
      setError(describe(failure));
    } finally {
      setBusy(false);
    }
  }

  const lobbies: LobbyActions = {
    create: (variant) =>
      void enter(async (token) => (await gameApi.createLobby(variant, token)).id),
    join: (lobbyId, seat) =>
      void enter(async (token) => {
        await gameApi.join(lobbyId, seat, token);
        return lobbyId;
      }),
    open: (lobbyId) => navigate(`/play/${lobbyId}`),
    get you() {
      return loadGuest()?.handle ?? null;
    },
    get busy() {
      return busy();
    },
    get error() {
      return error();
    },
  };

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
              <>
                <Hero detail={detail()} />
                <Show when={detail().children.length > 0}>
                  <Variants detail={detail()} />
                </Show>
                <div class="game-sections">
                  <SectionRenderer detail={detail()} lobbies={lobbies} />
                </div>
              </>
            )}
          </Match>
          <Match when={true}>
            <p class="feed-message">Loading game…</p>
          </Match>
        </Switch>
      </main>
      <Footer />
    </div>
  );
}

function Hero(props: { detail: GameDetail }) {
  const page = () => props.detail.page;
  const accent = () => page().accent ?? FALLBACK_ACCENT;

  return (
    <section class="game-hero">
      <Picture detail={props.detail} accent={accent()} />
      <div class="game-hero-content">
        <nav class="breadcrumb" aria-label="Breadcrumb">
          <A href="/" class="breadcrumb-parent">Games</A>
          <span class="breadcrumb-sep">›</span>
          <Show when={props.detail.parent}>
            {(parent) => (
              <>
                <A href={`/games/${parent().slug}`} class="breadcrumb-parent">
                  {parent().name}
                </A>
                <span class="breadcrumb-sep">›</span>
              </>
            )}
          </Show>
          <span class="breadcrumb-current">{page().name}</span>
        </nav>
        <h1 class="game-page-title">{page().name}</h1>
        <div class="game-tags">
          <For each={page().tags}>{(tag) => <span class="game-tag">{tag}</span>}</For>
        </div>
        <p class="game-desc">{page().description}</p>
      </div>
    </section>
  );
}

/** Beside the title: the cover when the game has one, else its board as a game starts, else a
 *  block in its accent. */
function Picture(props: { detail: GameDetail; accent: string }) {
  const page = () => props.detail.page;

  return (
    <Switch
      fallback={
        <div
          class="game-hero-image"
          style={{ background: `linear-gradient(135deg, ${props.accent}, #1A0A00)` }}
        />
      }
    >
      <Match when={page().cover_image}>
        {(src) => (
          <div class="game-hero-image">
            <img src={src()} alt={page().name} />
          </div>
        )}
      </Match>
      <Match when={props.detail.board}>
        {(board) => (
          <div class="game-hero-board">
            <GameBoard view={board()} skin={page().skin} />
          </div>
        )}
      </Match>
    </Switch>
  );
}

/** The games that are variants of this one, given room of their own right under the title. */
function Variants(props: { detail: GameDetail }) {
  const count = () => props.detail.children.length;

  return (
    <section id="variants" class="game-variants" aria-labelledby="variants-title">
      <h2 id="variants-title" class="feed-title">
        Variants
        <span class="variants-count">
          {count()} {count() === 1 ? "game" : "games"} built on {props.detail.page.name}
        </span>
      </h2>
      <div class="game-grid">
        <For each={props.detail.children}>{(child) => <CatalogCard entry={child} />}</For>
      </div>
    </section>
  );
}
