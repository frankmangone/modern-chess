import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { AuthProvider } from "../auth/provider";
import { configureAuth } from "../auth/session";
import { forgetGuest, saveGuest } from "../auth/token-store";
import { json, stubFetch } from "../test/server";
import { SharedGamePage } from "./shared-game";

vi.mock("../components/board-loader", () => ({ loadBoardElement: vi.fn().mockResolvedValue(undefined) }));
const engine = vi.hoisted(() => ({ checkSpec: vi.fn() }));
vi.mock("../builder/engine", () => engine);

const SECRET = "SECRETSECRETSECRETSECRE";
const game = { name: "Stride", rules: JSON.stringify({ name: "STRIDE" }), skin: "" };

function show() {
  const history = createMemoryHistory();
  history.set({ value: `/build/shared/${SECRET}` });
  render(() => (
    <MemoryRouter history={history}>
      <Route path="/build/shared/:token" component={SharedGamePage} />
      <Route path="/build" component={() => <p>the workbench</p>} />
    </MemoryRouter>
  ));
  return history;
}

const playing = () =>
  JSON.parse(document.querySelector(".shared-play modern-chess-board script")?.textContent ?? "null") as { name?: string } | null;

beforeEach(async () => {
  localStorage.clear();
  forgetGuest();
  await configureAuth(null);
  engine.checkSpec.mockReset();
  engine.checkSpec.mockImplementation((text: string) => {
    try {
      const spec = JSON.parse(text) as { name?: string; broken?: unknown };
      if ("broken" in spec) return Promise.resolve({ ok: false, message: "unknown field `broken`", line: 2, column: 3, path: null });
      return Promise.resolve({ ok: true, name: spec.name ?? "?", players: ["A", "B"] });
    } catch {
      return Promise.resolve({ ok: false, message: "expected value", line: 1, column: 1, path: null });
    }
  });
});

describe("A game shared by a private link", () => {
  it("opens with no sign-in, names the game, and plays it", async () => {
    const calls = stubFetch({ [`GET /api/game/shared/${SECRET}`]: () => json(game) });
    show();

    expect(await screen.findByRole("heading", { name: "Stride" })).toBeTruthy();
    await waitFor(() => expect(playing()?.name).toBe("STRIDE"));
    expect(calls[0].authorization).toBeNull();
  });

  it("shows the rules, read only, under a heading of their own", async () => {
    stubFetch({ [`GET /api/game/shared/${SECRET}`]: () => json(game) });
    show();
    await screen.findByRole("heading", { name: "Stride" });
    await waitFor(() => expect(playing()).not.toBeNull());

    const summary = screen.getByText("See the rules");
    expect(summary.closest("details")?.querySelector("pre")?.textContent).toBe(game.rules);
    expect(document.querySelector("textarea")).toBeNull();
  });

  it("says a link that opens nothing opens nothing, and does not say who or why", async () => {
    stubFetch({ [`GET /api/game/shared/${SECRET}`]: () => json({ error: "this link does not open anything" }, 404) });
    show();

    expect(await screen.findByRole("heading", { name: "This link does not open a game" })).toBeTruthy();
    expect(screen.getByRole("link", { name: "Open the workbench" }).getAttribute("href")).toBe("/build");
    expect(document.querySelector("modern-chess-board")).toBeNull();
  });

  it("says when the server cannot be reached", async () => {
    stubFetch({ [`GET /api/game/shared/${SECRET}`]: () => json({ error: "something went wrong" }, 500) });
    show();

    expect(await screen.findByRole("heading", { name: "The game could not be opened" })).toBeTruthy();
  });

  it("does not play a game the engine cannot read, and says what is wrong", async () => {
    stubFetch({ [`GET /api/game/shared/${SECRET}`]: () => json({ ...game, rules: JSON.stringify({ broken: 1 }) }) });
    show();

    expect(await screen.findByText(/The engine cannot read this game/)).toBeTruthy();
    expect(screen.getByRole("alert").textContent).toContain("unknown field `broken`");
    expect(document.querySelector("modern-chess-board")).toBeNull();
  });

  it("does not play a game whose look cannot be read", async () => {
    stubFetch({ [`GET /api/game/shared/${SECRET}`]: () => json({ ...game, skin: "{ nope" }) });
    show();

    expect(await screen.findByText(/The look of this game cannot be read/)).toBeTruthy();
    expect(document.querySelector("modern-chess-board")).toBeNull();
  });

  it("does not start by itself a game that did not finish starting last time", async () => {
    localStorage.setItem("hubbable.shared-run", SECRET);
    stubFetch({ [`GET /api/game/shared/${SECRET}`]: () => json(game) });
    show();

    expect(await screen.findByText(/did not finish starting last time/)).toBeTruthy();
    expect(document.querySelector("modern-chess-board")).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Start it anyway" }));

    await waitFor(() => expect(playing()?.name).toBe("STRIDE"));
  });

  it("makes a copy in the workbench, in this browser, when nobody is signed in", async () => {
    stubFetch({ [`GET /api/game/shared/${SECRET}`]: () => json(game) });
    show();
    fireEvent.click(await screen.findByRole("button", { name: "Make my own copy" }));

    expect(await screen.findByText("the workbench")).toBeTruthy();
    const kept = JSON.parse(localStorage.getItem("hubbable.drafts") ?? "null") as {
      drafts: { name: string; rules: string; sync: { kind: string } }[];
    };
    expect(kept.drafts).toHaveLength(1);
    expect(kept.drafts[0]).toMatchObject({ name: "Stride", rules: game.rules, sync: { kind: "local" } });
  });

  it("makes the copy for the account when someone is signed in, and numbers a name already taken", async () => {
    const exp = Math.floor(Date.now() / 1000) + 600;
    saveGuest({ user: { id: "u1", handle: "Ada", setup_pending: false }, token: `x.${btoa(JSON.stringify({ exp }))}.y` }, true);
    const p: AuthProvider = {
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
    await configureAuth(p);
    localStorage.setItem(
      "hubbable.drafts",
      JSON.stringify({ drafts: [{ id: "d", name: "Stride", rules: "{}", skin: "", updated: 1 }], current: "d" }),
    );
    stubFetch({ [`GET /api/game/shared/${SECRET}`]: () => json(game) });
    show();

    fireEvent.click(await screen.findByRole("button", { name: "Make my own copy" }));
    await screen.findByText("the workbench");

    const kept = JSON.parse(localStorage.getItem("hubbable.drafts") ?? "null") as {
      drafts: { name: string; sync: { kind: string; version: number | null } }[];
    };
    expect(kept.drafts.map((d) => d.name)).toEqual(["Stride", "Stride 2"]);
    expect(kept.drafts[1].sync).toEqual({ kind: "dirty", version: null });
  });
});
