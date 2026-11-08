import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { fireEvent, render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import { detail, fakeToken } from "../test/fixtures";
import { json, stubFetch } from "../test/server";
import { GameDetailPage } from "./game";

vi.mock("../components/board-loader", () => ({ loadBoardElement: vi.fn().mockResolvedValue(undefined) }));

const GAME = "GET /api/game/catalog/chess";
const GUESTS = "POST /api/social/guests";
const CREATE = "POST /api/game/catalog/chess/lobbies";
const LOBBY = "11111111-1111-1111-1111-111111111111";
const JOIN = `POST /api/game/instances/${LOBBY}/join`;
const NEW_MATCH = "22222222-2222-2222-2222-222222222222";
const STORE_KEY = "modern-chess.guest";

function guest(token: string) {
  return json({ user: { id: "u1", handle: "Guest-4242" }, token }, 201);
}

function open() {
  const history = createMemoryHistory();
  history.set({ value: "/games/chess" });
  render(() => (
    <MemoryRouter history={history}>
      <Route path="/games/:slug" component={GameDetailPage} />
      <Route path="/play/:id" component={() => <p>the play page</p>} />
    </MemoryRouter>
  ));
  return history;
}

function remember(token: string) {
  localStorage.setItem(STORE_KEY, JSON.stringify({ token, handle: "Guest-1" }));
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("GameDetailPage", () => {
  it("create_lobby_makes_a_guest_first_when_none_exists", async () => {
    const token = fakeToken(3600);
    const calls = stubFetch({
      [GAME]: () => json(detail()),
      [GUESTS]: () => guest(token),
      [CREATE]: () => json({ id: NEW_MATCH }, 201),
    });
    open();

    fireEvent.click(await screen.findByRole("button", { name: /create lobby/i }));

    expect(await screen.findByText("the play page")).toBeTruthy();
    expect(calls.map((c) => c.route)).toEqual([GAME, GUESTS, CREATE]);
    expect(calls[2].authorization).toBe(`Bearer ${token}`);
  });

  it("create_lobby_reuses_an_existing_guest", async () => {
    const token = fakeToken(3600);
    remember(token);
    const calls = stubFetch({
      [GAME]: () => json(detail()),
      [CREATE]: () => json({ id: NEW_MATCH }, 201),
    });
    open();

    fireEvent.click(await screen.findByRole("button", { name: /create lobby/i }));

    await screen.findByText("the play page");
    expect(calls.map((c) => c.route)).toEqual([GAME, CREATE]);
    expect(calls[1].authorization).toBe(`Bearer ${token}`);
  });

  it("an_expired_token_is_replaced_by_a_new_guest", async () => {
    remember(fakeToken(-10));
    const fresh = fakeToken(3600);
    const calls = stubFetch({
      [GAME]: () => json(detail()),
      [GUESTS]: () => guest(fresh),
      [CREATE]: () => json({ id: NEW_MATCH }, 201),
    });
    open();

    fireEvent.click(await screen.findByRole("button", { name: /create lobby/i }));

    await screen.findByText("the play page");
    expect(calls.map((c) => c.route)).toEqual([GAME, GUESTS, CREATE]);
    expect(calls[2].authorization).toBe(`Bearer ${fresh}`);
    expect(JSON.parse(localStorage.getItem(STORE_KEY) ?? "{}").token).toBe(fresh);
  });

  it("a_token_the_server_rejects_is_replaced_and_the_call_tried_once_more", async () => {
    remember(fakeToken(3600));
    const fresh = fakeToken(7200);
    let attempts = 0;
    const calls = stubFetch({
      [GAME]: () => json(detail()),
      [GUESTS]: () => guest(fresh),
      [CREATE]: () =>
        ++attempts === 1 ? json({ error: "sign in required" }, 401) : json({ id: NEW_MATCH }, 201),
    });
    open();

    fireEvent.click(await screen.findByRole("button", { name: /create lobby/i }));

    await screen.findByText("the play page");
    expect(calls.map((c) => c.route)).toEqual([GAME, CREATE, GUESTS, CREATE]);
    expect(calls[3].authorization).toBe(`Bearer ${fresh}`);
  });

  it("join_navigates_to_the_play_page", async () => {
    remember(fakeToken(3600));
    const calls = stubFetch({
      [GAME]: () => json(detail()),
      [JOIN]: () => json({ id: LOBBY }),
    });
    const history = open();

    fireEvent.click(await screen.findByRole("button", { name: /^join/i }));

    await screen.findByText("the play page");
    expect(history.get()).toBe(`/play/${LOBBY}`);
    expect(calls[1].body).toEqual({ seat: "black" });
  });

  it("a_failed_create_shows_the_reason_and_stays_on_the_page", async () => {
    remember(fakeToken(3600));
    stubFetch({
      [GAME]: () => json(detail()),
      [CREATE]: () => json({ error: "game not found" }, 404),
    });
    open();

    fireEvent.click(await screen.findByRole("button", { name: /create lobby/i }));

    expect((await screen.findByRole("alert")).textContent).toContain("game not found");
    expect(screen.queryByText("the play page")).toBeNull();
  });

  it("says so when the game does not exist", async () => {
    stubFetch({ [GAME]: () => json({ error: "game not found" }, 404) });
    open();

    expect((await screen.findByRole("alert")).textContent).toMatch(/couldn't find this game/i);
  });
});
