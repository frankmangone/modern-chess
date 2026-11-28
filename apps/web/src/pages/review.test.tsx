import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { fireEvent, render, screen, waitFor, within } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AuthProvider } from "../auth/provider";
import { configureAuth } from "../auth/session";
import { forgetGuest, saveGuest } from "../auth/token-store";
import type { ReviewGame } from "../api/types";
import { json, stubFetch } from "../test/server";
import { ReviewPage } from "./review";

const engine = vi.hoisted(() => ({ checkSpec: vi.fn() }));
vi.mock("../builder/engine", () => engine);
// The board draws only in a browser with the board element; this stands in for it.
vi.mock("../builder/playground", () => ({
  Playground: (props: { onAlive: () => void }) => (
    <button type="button" onClick={props.onAlive}>
      the board draws
    </button>
  ),
}));

function token(): string {
  return `x.${btoa(JSON.stringify({ exp: Math.floor(Date.now() / 1000) + 600 }))}.y`;
}

function provider(): AuthProvider {
  return {
    name: "test",
    start: () => Promise.resolve(),
    hasSession: () => true,
    getToken: () => Promise.resolve("t"),
    google: false,
    sendCode: () => Promise.resolve(),
    verifyCode: () => Promise.resolve(),
    continueWithGoogle: () => Promise.resolve(),
    completeRedirect: () => Promise.resolve("/"),
    signOut: () => Promise.resolve(),
    deleteUser: () => Promise.resolve(),
    subscribe: () => () => undefined,
  };
}

const LISTING = { tagline: "Two in a row", description: "A game for two.", licence: "CC-BY-SA-4.0" };
const LINE = { id: "g1", name: "Tic Tac", author: "Ada", listing: LISTING, submitted_at: "2026-11-27T10:00:00Z" };
const GAME: ReviewGame = { ...LINE, rules: '{"name":"TIC"}', skin: "" };

async function signedIn() {
  saveGuest({ user: { id: "u1", handle: "Root", setup_pending: false }, token: token() }, true);
  await configureAuth(provider());
}

function show() {
  const history = createMemoryHistory();
  history.set({ value: "/review" });
  render(() => (
    <MemoryRouter history={history}>
      <Route path="/review" component={ReviewPage} />
    </MemoryRouter>
  ));
}

beforeEach(async () => {
  localStorage.clear();
  forgetGuest();
  await configureAuth(null);
  engine.checkSpec.mockReset();
  engine.checkSpec.mockResolvedValue({ ok: true, name: "TIC", players: ["A", "B"] });
});

describe("Review", () => {
  it("asks a signed-out visitor to sign in, and asks the server nothing", async () => {
    const calls = stubFetch({});
    show();

    expect(await screen.findByRole("link", { name: "Sign in" })).toBeTruthy();
    expect(calls).toHaveLength(0);
  });

  it("tells someone who is not staff that the page is not for them", async () => {
    await signedIn();
    stubFetch({ "GET /api/game/review": () => json({ error: "That is not yours to do." }, 403) });
    show();

    expect(await screen.findByText("This page is for the people who look at new games.")).toBeTruthy();
  });

  it("says when nothing is waiting", async () => {
    await signedIn();
    stubFetch({ "GET /api/game/review": () => json([]) });
    show();

    expect(await screen.findByText("Nothing is waiting.")).toBeTruthy();
  });

  it("lists the games waiting, and shows one with its words, and plays it", async () => {
    await signedIn();
    const calls = stubFetch({
      "GET /api/game/review": () => json([LINE]),
      "GET /api/game/review/g1": () => json(GAME),
    });
    show();

    fireEvent.click(await screen.findByRole("button", { name: /Tic Tac/ }));

    expect(await screen.findByRole("heading", { name: "Tic Tac" })).toBeTruthy();
    expect(screen.getByText("Two in a row")).toBeTruthy();
    expect(screen.getByText("A game for two.")).toBeTruthy();
    expect(screen.getByText(/By Ada, under CC-BY-SA-4.0/)).toBeTruthy();
    expect(await screen.findByRole("button", { name: "the board draws" })).toBeTruthy();
    expect(calls.find((c) => c.route === "GET /api/game/review/g1")?.authorization).toMatch(/^Bearer /);
  });

  it("lets a game through, and takes it off the list", async () => {
    await signedIn();
    let waiting = [LINE];
    const calls = stubFetch({
      "GET /api/game/review": () => json(waiting),
      "GET /api/game/review/g1": () => json(GAME),
      "POST /api/game/review/g1/approve": () => {
        waiting = [];
        return json({ slug: "tic-tac" });
      },
    });
    show();
    fireEvent.click(await screen.findByRole("button", { name: /Tic Tac/ }));
    const approve = await screen.findByRole("button", { name: "Let it through" });
    await waitFor(() => expect((approve as HTMLButtonElement).disabled).toBe(false));

    fireEvent.click(approve);

    expect(await screen.findByText("“Tic Tac” is on the hub.")).toBeTruthy();
    expect(await screen.findByText("Nothing is waiting.")).toBeTruthy();
    expect(calls.some((c) => c.route === "POST /api/game/review/g1/approve")).toBe(true);
  });

  it("will not let through a game the engine cannot read", async () => {
    await signedIn();
    engine.checkSpec.mockResolvedValue({ ok: false, message: "expected value", line: 1, column: 1, path: null });
    stubFetch({ "GET /api/game/review": () => json([LINE]), "GET /api/game/review/g1": () => json(GAME) });
    show();
    fireEvent.click(await screen.findByRole("button", { name: /Tic Tac/ }));

    await screen.findByText(/The engine cannot read this game/);

    await waitFor(() => expect((screen.getByRole("button", { name: "Let it through" }) as HTMLButtonElement).disabled).toBe(true));
  });

  it("sends a game back only with a reason, and sends the reason", async () => {
    await signedIn();
    let waiting = [LINE];
    const calls = stubFetch({
      "GET /api/game/review": () => json(waiting),
      "GET /api/game/review/g1": () => json(GAME),
      "POST /api/game/review/g1/reject": () => {
        waiting = [];
        return new Response(null, { status: 204 });
      },
    });
    show();
    fireEvent.click(await screen.findByRole("button", { name: /Tic Tac/ }));
    fireEvent.click(await screen.findByRole("button", { name: "Send it back…" }));

    const send = screen.getByRole("button", { name: "Send it back" }) as HTMLButtonElement;
    expect(send.disabled).toBe(true);
    fireEvent.input(screen.getByLabelText(/Why it goes back/), { target: { value: "Say what it is" } });
    expect(send.disabled).toBe(false);
    fireEvent.click(send);

    expect(await screen.findByText("“Tic Tac” went back to its author.")).toBeTruthy();
    expect(calls.find((c) => c.route === "POST /api/game/review/g1/reject")?.body).toEqual({ reason: "Say what it is" });
  });

  it("says why when the server refuses, and keeps the game on show", async () => {
    await signedIn();
    stubFetch({
      "GET /api/game/review": () => json([LINE]),
      "GET /api/game/review/g1": () => json(GAME),
      "POST /api/game/review/g1/approve": () => json({ error: "The game is not in a state where that can be done." }, 409),
    });
    show();
    fireEvent.click(await screen.findByRole("button", { name: /Tic Tac/ }));
    const approve = await screen.findByRole("button", { name: "Let it through" });
    await waitFor(() => expect((approve as HTMLButtonElement).disabled).toBe(false));

    fireEvent.click(approve);

    expect(await screen.findByText("The game is not in a state where that can be done.")).toBeTruthy();
    expect(within(screen.getByRole("list", { name: "Games waiting" })).getByText("Tic Tac")).toBeTruthy();
  });
});
