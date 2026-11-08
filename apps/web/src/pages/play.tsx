import { A, useParams } from "@solidjs/router";
import { For, Match, Show, Switch, createResource, createSignal, onCleanup, onMount } from "solid-js";
import { gameApi } from "../api/game";
import { ApiError } from "../api/http";
import { startPolling, type Poller } from "../api/poll";
import type { Command, InstanceState } from "../api/types";
import { withGuest } from "../auth/identity";
import { loadGuest } from "../auth/token-store";
import Footer from "../components/Footer";
import Navbar from "../components/Navbar";
import { RemoteBoard } from "../components/remote-board";

const OUT_OF_REACH = "We can't reach the game server right now. Trying again…";

function describe(error: unknown): string {
  // A 5xx is the server (or the proxy in front of it) failing, not an answer meant for a player.
  if (error instanceof ApiError && error.status < 500) return error.message;
  return OUT_OF_REACH;
}

const SEAT_LOST =
  "You can only watch this match now: your guest sign-in has expired. Moves from this window are no longer accepted.";

function freeSeat(state: InstanceState): string | undefined {
  const taken = new Set(state.players.map((player) => player.seat));
  return state.seats.find((seat) => !taken.has(seat));
}

/** One sentence about where the match stands, for the person looking at it. */
function headline(state: InstanceState): string {
  const { status, outcome, you, view, players } = state;
  if (status === "lobby") return "Waiting for another player to join.";
  if (status === "finished") {
    if (outcome?.type === "won") return outcome.seat === you ? "You won!" : you === null ? `${outcome.seat} won.` : "You lost.";
    return "It's a draw.";
  }
  if (you === null) return `${view.turn} to move.`;
  if (view.turn === you) return "Your turn.";
  const opponent = players.find((player) => player.seat === view.turn);
  return `Waiting for ${opponent?.handle ?? view.turn}…`;
}

export function PlayPage() {
  const params = useParams<{ id: string }>();
  const [state, setState] = createSignal<InstanceState | null>(null);
  const [problem, setProblem] = createSignal<string | null>(null);
  const [missing, setMissing] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  let poller: Poller | undefined;
  let hadSeat = false;

  onMount(() => {
    poller = startPolling({
      load: (since) => gameApi.getInstance(params.id, since, loadGuest()?.token),
      onState: (next) => {
        setState(next);
        // A lapsed token turns a player into a spectator without any error; say so, and keep
        // saying it for as long as they are only watching.
        if (next.you !== null) hadSeat = true;
        setProblem(hadSeat && next.you === null ? SEAT_LOST : null);
      },
      onError: (error) => {
        if (error instanceof ApiError && error.status === 404) {
          setMissing(true);
          poller?.stop();
        } else if (error !== null) {
          setProblem(describe(error));
        }
      },
    });
  });
  onCleanup(() => poller?.stop());

  // The page for the game gives the skin and the name; the board looks plain without it.
  const [game] = createResource(
    () => state()?.slug,
    (slug) => gameApi.getGame(slug).catch(() => null),
  );

  // In a two-player game the second player sits at the other side of the board.
  const flipped = () => {
    const current = state();
    return current !== null && current.seats.length === 2 && current.you === current.seats[1];
  };

  const interactive = () => {
    const current = state();
    return !busy() && current !== null && current.status === "active" && current.you === current.view.turn;
  };

  async function run(task: (token: string) => Promise<InstanceState>) {
    setBusy(true);
    // Whatever was said about the last attempt is over once the player tries something else.
    setProblem(null);
    try {
      const next = await withGuest(task);
      poller?.accept(next);
      setState(next);
      setProblem(null);
    } catch (error) {
      // A stale position is not the player's mistake: look again and let the board catch up.
      if (error instanceof ApiError && error.status === 409) poller?.refresh();
      else setProblem(describe(error));
    } finally {
      setBusy(false);
    }
  }

  function send(kind: Command) {
    const current = state();
    if (current === null || !interactive()) return;
    void run((token) => gameApi.sendCommand(params.id, current.revision, kind, token));
  }

  function takeSeat() {
    const current = state();
    const seat = current === null ? undefined : freeSeat(current);
    if (seat === undefined) return;
    void run(async (token) => {
      await gameApi.join(params.id, seat, token);
      return (await gameApi.getInstance(params.id, undefined, token)) ?? current!;
    });
  }

  return (
    <div>
      <Navbar activeLink="games" />
      <Switch>
        <Match when={missing()}>
          <p class="feed-message" role="alert">We couldn't find this match.</p>
        </Match>
        <Match when={state()}>
          {(current) => (
            <main class="play">
              <p class="breadcrumb">
                <A href={`/games/${current().slug}`} class="breadcrumb-parent">
                  ← {game()?.spec.name ?? current().slug}
                </A>
              </p>
              <h1 class="play-headline">{headline(current())}</h1>
              <Show when={problem()}>
                {(message) => <p class="feed-message" role="alert">{message()}</p>}
              </Show>
              <div class="play-layout">
                <RemoteBoard
                  view={current().view}
                  skin={game()?.page.skin ?? null}
                  interactive={interactive()}
                  flipped={flipped()}
                  onCommand={send}
                />
                <ul class="seats">
                  <For each={current().seats}>
                    {(seat) => {
                      const player = () => current().players.find((p) => p.seat === seat);
                      return (
                        <li class="seat" classList={{ "seat-turn": current().status === "active" && current().view.turn === seat }}>
                          <span class="seat-name">{seat}</span>
                          <Show when={player()} fallback={<span class="seat-empty">Waiting for a player…</span>}>
                            {(taken) => (
                              <span>
                                {taken().handle}
                                {current().you === seat ? " (you)" : ""}
                              </span>
                            )}
                          </Show>
                        </li>
                      );
                    }}
                  </For>
                  <Show when={current().status === "lobby" && current().you === null && freeSeat(current())}>
                    <button class="btn-create-game" disabled={busy()} onClick={takeSeat}>
                      Take a seat
                    </button>
                  </Show>
                </ul>
              </div>
            </main>
          )}
        </Match>
        <Match when={problem()}>
          {(message) => <p class="feed-message" role="alert">{message()}</p>}
        </Match>
        <Match when={true}>
          <p class="feed-message">Loading match…</p>
        </Match>
      </Switch>
      <Footer />
    </div>
  );
}
