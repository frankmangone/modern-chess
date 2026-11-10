import { useParams } from "@solidjs/router";
import { For, Match, Show, Switch, createEffect, createResource, createSignal, onCleanup, onMount } from "solid-js";
import { gameApi } from "../api/game";
import { ApiError } from "../api/http";
import { startPolling, type Poller } from "../api/poll";
import type { Command, InstanceState } from "../api/types";
import { withGuest } from "../auth/identity";
import { loadGuest } from "../auth/token-store";
import Footer from "../components/Footer";
import { CapturedPanel } from "../components/captured-panel";
import { ChatPlaceholder } from "../components/chat-placeholder";
import Navbar from "../components/Navbar";
import { PlayBar } from "../components/play-bar";
import { PlayIsland } from "../components/play-island";
import { PlayNav } from "../components/play-nav";
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
  /** The piece picked from the hand, waiting for a square. */
  const [dropCode, setDropCode] = createSignal<string | null>(null);
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

  // A picked piece is forgotten as soon as it is not the player's move any more.
  createEffect(() => {
    if (!interactive()) setDropCode(null);
  });

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
    setDropCode(null);
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

  const name = () => game()?.spec.name ?? state()?.slug ?? "";
  /** The viewer's side sits at the bottom; a spectator sees the first seat there. */
  const bottomSeat = (current: InstanceState) => current.you ?? current.seats[0] ?? "";
  const handleOf = (current: InstanceState, seat: string) =>
    current.players.find((player) => player.seat === seat)?.handle ?? null;

  const bar = (current: () => InstanceState, seat: string) => (
    <PlayBar
      seat={seat}
      handle={handleOf(current(), seat)}
      you={current().you === seat}
      active={current().status === "active" && current().view.turn === seat}
    >
      <Show when={current().status === "lobby" && current().you === null && freeSeat(current()) === seat}>
        <button class="btn-create-game" disabled={busy()} onClick={takeSeat}>
          Take a seat
        </button>
      </Show>
    </PlayBar>
  );

  return (
    <Switch>
      <Match when={state()}>
        {(current) => (
          <div class="play-screen">
            <PlayNav slug={current().slug} name={name()} status={headline(current())} />
            <main class="play-main">
              <CapturedPanel view={current().view} skin={game()?.page.skin ?? null} />
              <div class="play-center">
                <div class="play-stack">
                  <Show when={problem()}>
                    {(message) => <p class="feed-message play-alert" role="alert">{message()}</p>}
                  </Show>
                  <For each={current().seats.filter((seat) => seat !== bottomSeat(current()))}>
                    {(seat) => bar(current, seat)}
                  </For>
                  <div class="play-board">
                    <RemoteBoard
                      view={current().view}
                      skin={game()?.page.skin ?? null}
                      interactive={interactive()}
                      flipped={flipped()}
                      dropCode={dropCode()}
                      onCommand={send}
                    />
                  </div>
                  {bar(current, bottomSeat(current()))}
                </div>
              </div>
              <div class="play-side">
                <Show when={current().status === "active"}>
                  <PlayIsland
                    view={current().view}
                    skin={game()?.page.skin ?? null}
                    seat={current().you}
                    interactive={interactive()}
                    dropCode={dropCode()}
                    onPickDrop={setDropCode}
                    onPromote={(code) => send({ type: "promote", code })}
                    onPass={() => send({ type: "pass" })}
                  />
                </Show>
                <ChatPlaceholder />
              </div>
            </main>
          </div>
        )}
      </Match>
      <Match when={true}>
        <div>
          <Navbar activeLink="games" />
          <Switch>
            <Match when={missing()}>
              <p class="feed-message" role="alert">We couldn't find this match.</p>
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
      </Match>
    </Switch>
  );
}
