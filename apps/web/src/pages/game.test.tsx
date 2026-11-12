import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { fireEvent, render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import { detail, fakeToken, goDetail } from "../test/fixtures";
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

function open(game = "chess") {
  const history = createMemoryHistory();
  history.set({ value: `/games/${game}` });
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

  it("a_proxy_failure_on_create_is_not_shown_as_raw_status_text", async () => {
    remember(fakeToken(3600));
    stubFetch({
      [GAME]: () => json(detail()),
      [CREATE]: () => new Response("<html>Bad Gateway</html>", { status: 502 }),
    });
    open();

    fireEvent.click(await screen.findByRole("button", { name: /create lobby/i }));

    const message = (await screen.findByRole("alert")).textContent ?? "";
    expect(message).toMatch(/something went wrong on our side/i);
    expect(message).not.toMatch(/502/);
  });

  it("says so when the game does not exist", async () => {
    stubFetch({ [GAME]: () => json({ error: "game not found" }, 404) });
    open();

    expect((await screen.findByRole("alert")).textContent).toMatch(/couldn't find this game/i);
  });

  it("your_own_lobby_opens_instead_of_offering_to_join_it", async () => {
    remember(fakeToken(3600));
    stubFetch({
      [GAME]: () =>
        json(
          detail({
            open_lobbies: [
              {
                id: LOBBY,
                variant: "chess",
                variant_label: "Chess",
                capacity: 2,
                players: [{ seat: "white", handle: "Guest-1" }],
                created_at: "2026-11-08T10:00:00Z",
              },
            ],
          }),
        ),
    });
    const history = open();

    expect(screen.queryByRole("button", { name: /^join/i })).toBeNull();
    fireEvent.click(await screen.findByRole("button", { name: /^open/i }));

    await screen.findByText("the play page");
    expect(history.get()).toBe(`/play/${LOBBY}`);
  });
});

describe("GameDetailPage with variants", () => {
  const GO = "GET /api/game/catalog/go";
  const lobby = (id: string, variant: string, label: string, players = [{ seat: "BLACK", handle: "Guest-9" }]) => ({
    id,
    variant,
    variant_label: label,
    capacity: 2,
    players,
    created_at: "2026-11-08T10:00:00Z",
  });

  it("the_hero_names_the_game_not_its_preview_rules", async () => {
    stubFetch({ [GO]: () => json(goDetail()) });
    open("go");

    expect((await screen.findByRole("heading", { level: 1 })).textContent).toBe("Go");
    expect(screen.queryByText("Go 9x9")).toBeNull();
  });

  it("a_single_variant_game_shows_no_picker_and_no_labels", async () => {
    remember(fakeToken(3600));
    const calls = stubFetch({
      [GAME]: () => json(detail()),
      [CREATE]: () => json({ id: NEW_MATCH }, 201),
    });
    open();

    expect(await screen.findByRole("button", { name: /create lobby/i })).toBeTruthy();
    expect(screen.queryByRole("radiogroup")).toBeNull();
    expect(screen.queryAllByTestId("lobby-variant")).toHaveLength(0);
    fireEvent.click(screen.getByRole("button", { name: /create lobby/i }));
    await screen.findByText("the play page");
    expect(calls.map((c) => c.route)).toEqual([GAME, CREATE]);
  });

  it("the_picker_lists_every_variant_and_starts_on_the_preview", async () => {
    stubFetch({ [GO]: () => json(goDetail({ preview_variant: "go-13" })) });
    open("go");

    const radios = await screen.findAllByRole("radio");
    expect(radios.map((r) => r.textContent)).toEqual(["9x9", "13x13", "19x19"]);
    expect(radios.map((r) => r.getAttribute("aria-checked"))).toEqual(["false", "true", "false"]);
  });

  it("create_lobby_opens_it_in_the_picked_variant", async () => {
    remember(fakeToken(3600));
    const calls = stubFetch({
      [GO]: () => json(goDetail()),
      "POST /api/game/catalog/go-19/lobbies": () => json({ id: NEW_MATCH }, 201),
    });
    open("go");

    fireEvent.click(await screen.findByRole("radio", { name: "19x19" }));
    fireEvent.click(screen.getByRole("button", { name: /create lobby/i }));

    await screen.findByText("the play page");
    expect(calls.map((c) => c.route)).toEqual([GO, "POST /api/game/catalog/go-19/lobbies"]);
  });

  it("lobbies_of_every_variant_are_listed_with_their_label", async () => {
    stubFetch({
      [GO]: () =>
        json(
          goDetail({
            open_lobbies: [lobby("a", "go-19", "19x19"), lobby("b", "go", "9x9")],
          }),
        ),
    });
    open("go");

    await screen.findAllByTestId("lobby");
    expect(screen.getAllByTestId("lobby-variant").map((n) => n.textContent)).toEqual(["19x19", "9x9"]);
  });

  it("joining_uses_a_seat_of_the_lobbys_own_variant", async () => {
    remember(fakeToken(3600));
    const calls = stubFetch({
      [GO]: () => json(goDetail({ open_lobbies: [lobby("a", "go-13", "13x13")] })),
      "POST /api/game/instances/a/join": () => json({}),
    });
    open("go");

    fireEvent.click(await screen.findByRole("button", { name: /^join/i }));

    await screen.findByText("the play page");
    expect(calls[1].body).toEqual({ seat: "WHITE" });
  });

  it("a_lobby_of_an_unknown_variant_still_renders_without_a_join_button", async () => {
    stubFetch({
      [GO]: () => json(goDetail({ open_lobbies: [lobby("a", "go-25", "25x25")] })),
    });
    open("go");

    expect(await screen.findByTestId("lobby")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /^join/i })).toBeNull();
  });

  it("marks_its_main_content_and_names_the_page_after_the_game", async () => {
    stubFetch({ [GAME]: () => json(detail()) });
    open();

    await screen.findByRole("button", { name: /create lobby/i });

    expect(screen.getByRole("main")).toBeTruthy();
    expect(document.title).toBe(`${detail().page.name} · BoardHub`);
  });
});
