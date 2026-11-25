import { useParams } from "@solidjs/router";
import { For, Match, Show, Switch, createResource, createSignal, onCleanup, onMount } from "solid-js";
import { gameApi } from "../api/game";
import { ApiError } from "../api/http";
import { startPolling, type Poller } from "../api/poll";
import type { Command, InstanceState } from "../api/types";
import { withGuest } from "../auth/identity";
import { loadGuest } from "../auth/token-store";
import Footer from "../components/Footer";
import { CapturedPanel, showsCaptured } from "../components/captured-panel";
import { ChatPlaceholder } from "../components/chat-placeholder";
import Navbar from "../components/Navbar";
import { usePageTitle } from "../components/page-title";
import { PlayBar } from "../components/play-bar";
import { PlayIsland } from "../components/play-island";
import { PlayNav } from "../components/play-nav";
import { ResignControl } from "../components/resign-control";
import { RemoteBoard } from "../components/remote-board";

const OUT_OF_REACH = "We can't reach the game server right now. Trying again…";

function describe(error: unknown): string {
  // A 5xx is the server (or the proxy in front of it) failing, not an answer meant for a player.
  if (error instanceof ApiError && error.status < 500) return error.message;
  return OUT_OF_REACH;
}

/** For something the player did (a move, taking a seat): nothing is retried for them, so say it
 *  was not done. A 4xx is the game's own answer and is shown as it is. */
const NOT_DONE = "That did not go through. Please try again in a moment.";

function describeAttempt(error: unknown): string {
  if (error instanceof ApiError && error.status < 500) return error.message;
  return NOT_DONE;
}

const SEAT_LOST =
  "You can only watch this match now: your guest sign-in has expired. Moves from this window are no longer accepted.";

function freeSeat(state: InstanceState): string | undefined {
  const taken = new Set(state.players.map((player) => player.seat));
  return state.seats.find((seat) => !taken.has(seat));
}

/** It is the viewer's move in a match being played. */
function isYourTurn(state: InstanceState): boolean {
  return state.status === "active" && state.you !== null && state.you === state.view.turn;
}

/** The viewer is one of the seats an all-at-once question waits on, in a match being played. */
function ownsAnAnswer(state: InstanceState): boolean {
  return state.status === "active" && state.you !== null && (state.view.waiting ?? []).includes(state.you);
}

/** What the viewer is offered, and the answers given so far, are the same in both: the other seat's
 *  answer changed nothing this seat's answer depends on. */
function sameAnswers(before: InstanceState, after: InstanceState): boolean {
  return JSON.stringify(before.view.choice) === JSON.stringify(after.view.choice);
}

/** The side that did not win, in a match between two. */
function loser(state: InstanceState, winner: string): string {
  return state.seats.find((seat) => seat !== winner) ?? "The other side";
}

/** One sentence about where the match stands, for the person looking at it. */
function headline(state: InstanceState): string {
  const { status, outcome, you, view, players } = state;
  if (status === "lobby") return "Waiting for another player to join.";
  if (status === "finished") {
    if (outcome?.type !== "won") return "It's a draw.";
    if (outcome.reason === "resigned") {
      if (you === null) return `${loser(state, outcome.seat)} resigned. ${outcome.seat} won.`;
      return outcome.seat === you ? "Your opponent resigned. You won!" : "You resigned.";
    }
    return outcome.seat === you ? "You won!" : you === null ? `${outcome.seat} won.` : "You lost.";
  }
  const waiting = view.waiting ?? [];
  if (waiting.length > 1 && you !== null && !waiting.includes(you)) {
    return `Waiting for ${waiting.map((seat) => players.find((player) => player.seat === seat)?.handle ?? seat).join(" and ")}…`;
  }
  if (you === null) return waiting.length > 1 ? `Waiting for ${waiting.join(" and ")}.` : `${view.turn} to move.`;
  if (view.turn === you) return waiting.length > 1 ? "Your answer is needed." : "Your turn.";
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

  // The page for the game gives the skin and the name; the board looks plain without it. It is
  // loaded by the match's game: the match's own slug is a variant, which may have no page.
  const [game] = createResource(
    () => state()?.game,
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
      else setProblem(describeAttempt(error));
    } finally {
      setBusy(false);
    }
  }

  function send(kind: Command) {
    const current = state();
    if (current === null || !interactive()) return;
    void run(async (token) => {
      try {
        return await gameApi.sendCommand(params.id, current.revision, kind, token);
      } catch (error) {
        // Two seats answering at once: one gets the position first and the other is told it has
        // changed. When this seat's own answer is untouched by that, it goes again, once, on the
        // new position, so the player does not have to click twice.
        if (!(error instanceof ApiError && error.status === 409) || !ownsAnAnswer(current)) throw error;
        const latest = await gameApi.getInstance(params.id, undefined, token);
        if (latest === null || !ownsAnAnswer(latest) || !sameAnswers(current, latest)) throw error;
        return await gameApi.sendCommand(params.id, latest.revision, kind, token);
      }
    });
  }

  /** Giving up is not about whose turn it is, so it does not wait for one. */
  function resign() {
    const current = state();
    if (current === null || busy()) return;
    void run((token) => gameApi.sendCommand(params.id, current.revision, { type: "resign" }, token));
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

  /** The variant being played, as the game's page describes it. */
  const variant = () => game()?.variants.find((candidate) => candidate.slug === state()?.slug);
  const name = () => game()?.page.name ?? state()?.slug ?? "";
  /** "Go · 13x13" for a game with several variants, the game's name alone otherwise. */
  const title = () => {
    const detail = game();
    if (detail === null || detail === undefined || detail.variants.length < 2) return name();
    return `${name()} · ${variant()?.label ?? state()?.variant_label ?? state()?.slug ?? ""}`;
  };
  // A tab left in the background says when the move comes back to the player.
  usePageTitle(() => {
    const current = state();
    const page = title() || undefined;
    return page !== undefined && current !== null && isYourTurn(current) ? `Your turn — ${page}` : page;
  });
  /** A variant's skin replaces the game's; without one the game's applies. */
  const skin = () => variant()?.skin ?? game()?.page.skin ?? null;
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
            <PlayNav
              game={current().game}
              name={name()}
              title={title()}
              status={headline(current())}
              yours={isYourTurn(current())}
            >
              <Show when={current().status === "active" && current().you !== null}>
                <ResignControl busy={busy()} onResign={resign} />
              </Show>
            </PlayNav>
            <main class="play-main">
              {/* Not while the game's page loads: its skin may turn the panel off, and it would flash. */}
              <Show when={!game.loading && showsCaptured(skin())}>
                <CapturedPanel view={current().view} skin={skin()} />
              </Show>
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
                      skin={skin()}
                      interactive={interactive()}
                      flipped={flipped()}
                      onCommand={send}
                    />
                  </div>
                  {bar(current, bottomSeat(current()))}
                </div>
              </div>
              <div class="play-side">
                <Show when={current().status === "active" || current().status === "finished"}>
                  <PlayIsland
                    view={current().view}
                    skin={skin()}
                    seat={current().you}
                    interactive={interactive()}
                    finished={current().status === "finished"}
                    onChoose={(answer) => send({ type: "choose", ...answer })}
                    onCancel={() => send({ type: "cancel" })}
                  />
                </Show>
                <ChatPlaceholder />
              </div>
            </main>
          </div>
        )}
      </Match>
      <Match when={true}>
        <div class="page">
          <Navbar activeLink="games" />
          <main>
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
          </main>
          <Footer />
        </div>
      </Match>
    </Switch>
  );
}
