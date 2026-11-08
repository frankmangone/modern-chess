import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { fireEvent, render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import { boardView, detail, fakeToken, snapshot } from "../test/fixtures";
import { json, stubFetch } from "../test/server";
import { PlayPage } from "./play";

// The real board element, so these tests also prove the page and the board fit together.
vi.mock("../components/board-loader", () => ({
  loadBoardElement: () => import("@board/modern-chess-board.js").then(() => undefined),
}));

const MATCH = "33333333-3333-3333-3333-333333333333";
const STATE = `GET /api/game/instances/${MATCH}`;
const COMMAND = `POST /api/game/instances/${MATCH}/commands`;
const JOIN = `POST /api/game/instances/${MATCH}/join`;
const GAME = "GET /api/game/catalog/chess";
const GUESTS = "POST /api/social/guests";

function open() {
  const history = createMemoryHistory();
  history.set({ value: `/play/${MATCH}` });
  render(() => (
    <MemoryRouter history={history}>
      <Route path="/play/:id" component={PlayPage} />
    </MemoryRouter>
  ));
}

function signedIn() {
  localStorage.setItem("modern-chess.guest", JSON.stringify({ token: fakeToken(3600), handle: "Guest-1" }));
}

const settle = () => vi.advanceTimersByTimeAsync(0);

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe("PlayPage", () => {
  it("waiting_room_shows_empty_seats_until_full", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    signedIn();
    let answer = snapshot({
      status: "lobby",
      revision: 1,
      started_at: null,
      players: [{ seat: "WHITE", handle: "Guest-1" }],
    });
    stubFetch({ [STATE]: () => json(answer), [GAME]: () => json(detail()) });
    open();
    await settle();

    expect(screen.getByText(/^Guest-1/)).toBeTruthy();
    expect(screen.getByText(/waiting for a player/i)).toBeTruthy();
    expect(screen.getByText(/waiting for another player/i)).toBeTruthy();

    answer = snapshot({ revision: 2 });
    await vi.advanceTimersByTimeAsync(1500);

    expect(screen.queryByText(/waiting for a player/i)).toBeNull();
    expect(screen.getByText(/^Guest-2/)).toBeTruthy();
    expect(screen.getByText(/your turn/i)).toBeTruthy();
  });

  it("a_click_sends_the_command_with_the_revision_it_is_looking_at", async () => {
    signedIn();
    const calls = stubFetch({
      [STATE]: () => json(snapshot({ revision: 7 })),
      [GAME]: () => json(detail()),
      [COMMAND]: () => json(snapshot({ revision: 8, view: boardView({ selected: [0, 0], destinations: [[0, 1]] }) })),
    });
    open();

    fireEvent.click(await screen.findByLabelText("WHITE ROOK at 0,0"));

    const sent = await vi.waitFor(() => {
      const call = calls.find((c) => c.route === COMMAND);
      if (call === undefined) throw new Error("not sent yet");
      return call;
    });
    expect(sent.body).toEqual({ revision: 7, kind: { type: "select", x: 0, y: 0 } });
    expect(sent.authorization).toMatch(/^Bearer /);
    expect(await screen.findByLabelText("empty square 0,1")).toBeTruthy();
  });

  it("a_409_refetches_instead_of_showing_an_error", async () => {
    signedIn();
    let reads = 0;
    const calls = stubFetch({
      [STATE]: () => json(snapshot({ revision: ++reads === 1 ? 3 : 4, view: boardView({ turn: "WHITE" }) })),
      [GAME]: () => json(detail()),
      [COMMAND]: () => json({ error: "the position has changed" }, 409),
    });
    open();

    fireEvent.click(await screen.findByLabelText("WHITE ROOK at 0,0"));

    await vi.waitFor(() => expect(calls.filter((c) => c.route === STATE)).toHaveLength(2));
    expect(screen.queryByRole("alert")).toBeNull();
    expect(calls.filter((c) => c.route === STATE)[1].query).toBe("since=3");
  });

  it("an_illegal_move_is_explained_without_engine_details", async () => {
    signedIn();
    stubFetch({
      [STATE]: () => json(snapshot()),
      [GAME]: () => json(detail()),
      [COMMAND]: () => json({ error: "that is not a legal move" }, 422),
    });
    open();

    fireEvent.click(await screen.findByLabelText("WHITE ROOK at 0,0"));

    expect((await screen.findByRole("alert")).textContent).toContain("that is not a legal move");
  });

  it("the board cannot be played on the other player's turn", async () => {
    signedIn();
    stubFetch({
      [STATE]: () => json(snapshot({ you: "WHITE", view: boardView({ turn: "BLACK" }) })),
      [GAME]: () => json(detail()),
    });
    open();

    const piece = (await screen.findByLabelText("BLACK ROOK at 2,2")) as HTMLButtonElement;

    expect(piece.disabled).toBe(true);
    expect(screen.getByText(/waiting for Guest-2/i)).toBeTruthy();
  });

  it("says who won and stops asking when the game is over", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    signedIn();
    const calls = stubFetch({
      [STATE]: () =>
        json(
          snapshot({
            status: "finished",
            outcome: { type: "won", seat: "WHITE" },
            view: boardView({ status: { state: "won", winner: "WHITE" } }),
          }),
        ),
      [GAME]: () => json(detail()),
    });
    open();
    await settle();
    await vi.advanceTimersByTimeAsync(10_000);

    expect(screen.getByText(/you won/i)).toBeTruthy();
    expect(calls.filter((c) => c.route === STATE)).toHaveLength(1);
  });

  it("a_visitor_can_take_a_free_seat_from_the_link", async () => {
    let seated = false;
    const calls = stubFetch({
      [STATE]: () =>
        json(
          snapshot({
            status: "lobby",
            started_at: null,
            you: null,
            players: [{ seat: "WHITE", handle: "Guest-1" }],
            revision: seated ? 2 : 1,
          }),
        ),
      [GAME]: () => json(detail()),
      [GUESTS]: () => json({ user: { id: "u2", handle: "Guest-9" }, token: fakeToken(3600) }, 201),
      [JOIN]: () => {
        seated = true;
        return json(snapshot());
      },
    });
    open();

    fireEvent.click(await screen.findByRole("button", { name: /take a seat/i }));

    await vi.waitFor(() => expect(calls.some((c) => c.route === JOIN)).toBe(true));
    expect(calls.find((c) => c.route === JOIN)?.body).toEqual({ seat: "BLACK" });
    expect(calls.map((c) => c.route).indexOf(GUESTS)).toBeLessThan(calls.map((c) => c.route).indexOf(JOIN));
  });

  it("says so when the match does not exist", async () => {
    stubFetch({ [STATE]: () => json({ error: "match not found" }, 404) });
    open();

    expect((await screen.findByRole("alert")).textContent).toMatch(/couldn't find this match/i);
  });
});
