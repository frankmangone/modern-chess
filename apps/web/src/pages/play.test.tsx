import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { fireEvent, render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { InstanceState } from "../api/types";
import { GO_SKIN_13, boardView, detail, fakeToken, goDetail, snapshot } from "../test/fixtures";
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

  it("an_action_the_server_could_not_handle_says_it_did_not_go_through", async () => {
    signedIn();
    stubFetch({
      [STATE]: () => json(snapshot()),
      [GAME]: () => json(detail()),
      [COMMAND]: () => json({ error: "the game engine could not handle this position" }, 500),
    });
    open();

    fireEvent.click(await screen.findByLabelText("WHITE ROOK at 0,0"));

    const message = (await screen.findByRole("alert")).textContent ?? "";
    expect(message).toMatch(/did not go through/i);
    expect(message).not.toMatch(/can't reach|trying again/i);
  });

  it("an_action_sent_while_the_connection_is_down_says_it_did_not_go_through", async () => {
    signedIn();
    stubFetch({
      [STATE]: () => json(snapshot()),
      [GAME]: () => json(detail()),
      [COMMAND]: () => {
        throw new TypeError("Failed to fetch");
      },
    });
    open();

    fireEvent.click(await screen.findByLabelText("WHITE ROOK at 0,0"));

    expect((await screen.findByRole("alert")).textContent).toMatch(/did not go through/i);
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

  it("says so, in plain words, when the server is down on first load", async () => {
    stubFetch({});
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new TypeError("Failed to fetch")));
    open();

    expect((await screen.findByRole("alert")).textContent).toMatch(/can't reach the game server/i);
    expect(screen.queryByText(/loading match/i)).toBeNull();
  });

  it("does not show a proxy's 500 page as if it were the game's own answer", async () => {
    stubFetch({ [STATE]: () => new Response("<html>Bad Gateway</html>", { status: 502 }) });
    open();

    const message = (await screen.findByRole("alert")).textContent ?? "";

    expect(message).toMatch(/can't reach the game server/i);
    expect(message).not.toMatch(/502/);
  });

  it("tells a player who loses their seat mid-game instead of going quiet", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    signedIn();
    let answer = snapshot({ revision: 3, you: "WHITE" });
    stubFetch({ [STATE]: () => json(answer), [GAME]: () => json(detail()) });
    open();
    await settle();
    expect(screen.queryByRole("alert")).toBeNull();

    answer = snapshot({ revision: 4, you: null });
    await vi.advanceTimersByTimeAsync(1500);

    expect(screen.getByRole("alert").textContent).toMatch(/sign-in.*expired|watching/i);
  });

  it("the_second_player_sees_their_own_pieces_at_the_bottom", async () => {
    signedIn();
    stubFetch({
      [STATE]: () => json(snapshot({ you: "BLACK", view: boardView({ turn: "WHITE" }) })),
      [GAME]: () => json(detail()),
    });
    open();

    await screen.findByLabelText("WHITE ROOK at 0,0");

    const labels = Array.from(document.querySelectorAll(".mcb-cell")).map((c) => c.getAttribute("aria-label"));
    expect(labels[0]).toBe("empty square 2,0");
  });

  it("the_first_player_and_spectators_keep_the_usual_view", async () => {
    stubFetch({ [STATE]: () => json(snapshot({ you: null })), [GAME]: () => json(detail()) });
    open();

    await screen.findByLabelText("WHITE ROOK at 0,0");

    const labels = Array.from(document.querySelectorAll(".mcb-cell")).map((c) => c.getAttribute("aria-label"));
    expect(labels[0]).toBe("empty square 0,2");
  });

  it("shows_what_the_server_says_each_side_has_lost", async () => {
    stubFetch({
      [STATE]: () =>
        json(
          snapshot({
            view: boardView({
              players: ["WHITE", "BLACK"],
              captured: { WHITE: { PAWN: 1 }, BLACK: { QUEEN: 1 } },
              material: { WHITE: 38, BLACK: 30 },
            }),
          }),
        ),
      [GAME]: () => json(detail()),
    });
    open();

    expect(await screen.findByLabelText("BLACK lost: QUEEN ×1")).toBeTruthy();
    expect(screen.getByLabelText("WHITE lost: PAWN ×1")).toBeTruthy();
    expect(screen.getByText("WHITE +8")).toBeTruthy();
  });

  it("marks_the_viewer_in_their_own_strip_under_the_board", async () => {
    signedIn();
    stubFetch({
      [STATE]: () => json(snapshot({ you: "BLACK", view: boardView({ turn: "WHITE" }) })),
      [GAME]: () => json(detail()),
    });
    open();

    const strips = Array.from(await screen.findAllByText(/^Guest-/)).map((node) => node.textContent);

    expect(strips).toEqual(["Guest-1", "Guest-2 (you)"]);
  });

  it("holds_a_place_for_the_chat_without_offering_it", async () => {
    stubFetch({ [STATE]: () => json(snapshot()), [GAME]: () => json(detail()) });
    open();

    expect(await screen.findByText(/coming soon/i)).toBeTruthy();
    expect((screen.getByRole("button", { name: "Send" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByLabelText("Message") as HTMLInputElement).disabled).toBe(true);
  });

  it("the_illegal_move_message_goes_away_when_the_player_tries_again", async () => {
    signedIn();
    let attempts = 0;
    stubFetch({
      [STATE]: () => json(snapshot()),
      [GAME]: () => json(detail()),
      [COMMAND]: () =>
        ++attempts === 1 ? json({ error: "that is not a legal move" }, 422) : new Promise<Response>(() => {}),
    });
    open();
    fireEvent.click(await screen.findByLabelText("WHITE ROOK at 0,0"));
    await screen.findByRole("alert");

    fireEvent.click(await screen.findByLabelText("WHITE ROOK at 0,0"));

    await vi.waitFor(() => expect(screen.queryByRole("alert")).toBeNull());
  });
});

describe("PlayPage with a hand", () => {
  /** A Shogi-like match: SENTE holds a bishop and may drop it on one square. */
  const shogiMatch = (overrides: Partial<InstanceState> = {}) =>
    snapshot({
      slug: "shogi",
      revision: 12,
      seats: ["SENTE", "GOTE"],
      players: [
        { seat: "SENTE", handle: "Guest-1" },
        { seat: "GOTE", handle: "Guest-2" },
      ],
      you: "SENTE",
      view: boardView({
        players: ["SENTE", "GOTE"],
        pieces: [{ x: 0, y: 0, code: "ROOK", player: "SENTE" }],
        turn: "SENTE",
        hands: true,
        hand: { SENTE: { BISHOP: 1 }, GOTE: {} },
        drops: { BISHOP: [[1, 1]] },
      }),
      ...overrides,
    });

  const sentCommands = (calls: ReturnType<typeof stubFetch>) => calls.filter((c) => c.route === COMMAND);

  it("a_hand_piece_is_picked_and_dropped_on_a_highlighted_square", async () => {
    signedIn();
    const calls = stubFetch({
      [STATE]: () => json(shogiMatch()),
      [GAME]: () => json(detail()),
      [COMMAND]: () => json(shogiMatch({ revision: 13 })),
    });
    open();

    fireEvent.click(await screen.findByRole("button", { name: "Drop BISHOP" }));
    await vi.waitFor(() => expect(document.querySelectorAll(".mcb-dest")).toHaveLength(1));
    fireEvent.click(await screen.findByLabelText("empty square 1,1"));

    await vi.waitFor(() => expect(sentCommands(calls)).toHaveLength(1));
    expect(sentCommands(calls)[0].body).toEqual({
      revision: 12,
      kind: { type: "drop", code: "BISHOP", x: 1, y: 1 },
    });
  });

  it("only_squares_that_are_highlighted_take_the_drop", async () => {
    signedIn();
    const calls = stubFetch({ [STATE]: () => json(shogiMatch()), [GAME]: () => json(detail()) });
    open();

    fireEvent.click(await screen.findByRole("button", { name: "Drop BISHOP" }));
    await vi.waitFor(() => expect(document.querySelectorAll(".mcb-dest")).toHaveLength(1));
    fireEvent.click(await screen.findByLabelText("empty square 2,2"));

    expect(sentCommands(calls)).toHaveLength(0);
  });

  it("picking_a_piece_on_the_board_puts_the_hand_piece_back", async () => {
    signedIn();
    const calls = stubFetch({
      [STATE]: () => json(shogiMatch()),
      [GAME]: () => json(detail()),
      [COMMAND]: () => new Promise<Response>(() => {}),
    });
    open();
    fireEvent.click(await screen.findByRole("button", { name: "Drop BISHOP" }));
    await vi.waitFor(() => expect(document.querySelectorAll(".mcb-dest")).toHaveLength(1));

    fireEvent.click(await screen.findByLabelText("SENTE ROOK at 0,0"));

    await vi.waitFor(() => expect(sentCommands(calls)).toHaveLength(1));
    expect(sentCommands(calls)[0].body).toEqual({ revision: 12, kind: { type: "select", x: 0, y: 0 } });
    // The pick is forgotten: its squares are no longer lit while the command is on its way.
    await vi.waitFor(() => expect(document.querySelectorAll(".mcb-dest")).toHaveLength(0));
  });

  it("a_promotion_is_answered_from_the_island_and_not_under_the_board", async () => {
    signedIn();
    const calls = stubFetch({
      [STATE]: () => json(snapshot({ revision: 5, view: boardView({ promotion: ["QUEEN", "ROOK"] }) })),
      [GAME]: () => json(detail()),
      [COMMAND]: () => new Promise<Response>(() => {}),
    });
    open();

    const queen = await screen.findByRole("button", { name: "Promote to QUEEN" });
    expect(document.querySelector(".mcb-promo")).toBeNull();
    fireEvent.click(queen);

    await vi.waitFor(() => expect(sentCommands(calls)).toHaveLength(1));
    expect(sentCommands(calls)[0].body).toEqual({ revision: 5, kind: { type: "promote", code: "QUEEN" } });
  });

  it("the_island_is_only_shown_while_the_match_is_being_played", async () => {
    stubFetch({
      [STATE]: () => json(snapshot({ status: "lobby", revision: 1, started_at: null })),
      [GAME]: () => json(detail()),
    });
    open();
    await screen.findByLabelText("WHITE ROOK at 0,0");

    expect(screen.queryByLabelText("What to do next")).toBeNull();
  });
});

describe("PlayPage with a supply", () => {
  const goMatch = (overrides: Partial<InstanceState> = {}) =>
    snapshot({
      slug: "go",
      revision: 5,
      seats: ["BLACK", "WHITE"],
      players: [
        { seat: "BLACK", handle: "Guest-1" },
        { seat: "WHITE", handle: "Guest-2" },
      ],
      you: "BLACK",
      view: boardView({
        players: ["BLACK", "WHITE"],
        pieces: [{ x: 0, y: 0, code: "STONE", player: "WHITE" }],
        turn: "BLACK",
        supply: ["STONE"],
        drops: { STONE: [[1, 1]] },
        can_pass: true,
        score: { BLACK: 0, WHITE: 8.5 },
      }),
      ...overrides,
    });
  const sent = (calls: ReturnType<typeof stubFetch>) => calls.filter((c) => c.route === COMMAND);

  it("a_stone_is_placed_by_clicking_a_legal_point", async () => {
    signedIn();
    const calls = stubFetch({
      [STATE]: () => json(goMatch()),
      [GAME]: () => json(detail()),
      [COMMAND]: () => json(goMatch({ revision: 6 })),
    });
    open();

    fireEvent.click(await screen.findByLabelText("empty square 1,1"));

    await vi.waitFor(() => expect(sent(calls)).toHaveLength(1));
    expect(sent(calls)[0].body).toEqual({
      revision: 5,
      kind: { type: "drop", code: "STONE", x: 1, y: 1 },
    });
  });

  it("passing_sends_a_pass_command_with_the_revision_being_looked_at", async () => {
    signedIn();
    const calls = stubFetch({
      [STATE]: () => json(goMatch()),
      [GAME]: () => json(detail()),
      [COMMAND]: () => json(goMatch({ revision: 6 })),
    });
    open();

    fireEvent.click(await screen.findByRole("button", { name: "Pass" }));

    await vi.waitFor(() => expect(sent(calls)).toHaveLength(1));
    expect(sent(calls)[0].body).toEqual({ revision: 5, kind: { type: "pass" } });
  });

  it("the_pass_button_is_gone_once_the_game_is_over", async () => {
    signedIn();
    stubFetch({
      [STATE]: () =>
        json(
          goMatch({
            status: "finished",
            outcome: { type: "won", seat: "WHITE" },
            view: boardView({ players: ["BLACK", "WHITE"], supply: ["STONE"], can_pass: false }),
          }),
        ),
      [GAME]: () => json(detail()),
    });
    open();

    await screen.findByText(/you lost/i);
    expect(screen.queryByRole("button", { name: "Pass" })).toBeNull();
  });
});

describe("PlayPage with variants", () => {
  const GO = "GET /api/game/catalog/go";
  const GO_13 = "GET /api/game/catalog/go-13";
  const variantMatch = (overrides: Partial<InstanceState> = {}) =>
    snapshot({
      slug: "go-13",
      game: "go",
      variant_label: "13x13",
      seats: ["BLACK", "WHITE"],
      players: [
        { seat: "BLACK", handle: "Guest-1" },
        { seat: "WHITE", handle: "Guest-2" },
      ],
      you: "BLACK",
      view: boardView({ players: ["BLACK", "WHITE"], turn: "BLACK" }),
      ...overrides,
    });
  const skinOf = () => {
    const url = document.querySelector("modern-chess-board")?.getAttribute("skin");
    return url === null || url === undefined
      ? null
      : JSON.parse(decodeURIComponent(url.replace("data:application/json,", "")));
  };
  const title = () => screen.getByRole("heading", { level: 1 }).textContent ?? "";
  const backLink = () => document.querySelector("a.play-back");

  it("the_match_page_loads_the_game_by_the_matchs_game_slug", async () => {
    signedIn();
    const calls = stubFetch({ [STATE]: () => json(variantMatch()), [GO]: () => json(goDetail()) });
    open();

    await screen.findByLabelText("WHITE ROOK at 0,0");

    expect(calls.map((c) => c.route)).toContain(GO);
    expect(calls.map((c) => c.route)).not.toContain(GO_13);
  });

  it("a_variant_match_is_titled_game_dot_variant_and_links_back_to_the_game", async () => {
    signedIn();
    stubFetch({ [STATE]: () => json(variantMatch()), [GO]: () => json(goDetail()) });
    open();

    await vi.waitFor(() => expect(title()).toContain("Go · 13x13"));
    expect(backLink()?.getAttribute("href")).toBe("/games/go");
    expect(backLink()?.textContent).toContain("Go");
    expect(backLink()?.textContent).not.toContain("13x13");
  });

  it("a_single_variant_match_is_titled_with_the_game_name_only", async () => {
    signedIn();
    stubFetch({ [STATE]: () => json(snapshot()), [GAME]: () => json(detail()) });
    open();

    await vi.waitFor(() => expect(title()).toBe("♟Chess — Match"));
    expect(title()).not.toContain("·");
    expect(backLink()?.getAttribute("href")).toBe("/games/chess");
  });

  it("the_match_uses_the_variants_skin_and_falls_back_to_the_games", async () => {
    signedIn();
    stubFetch({ [STATE]: () => json(variantMatch()), [GO]: () => json(goDetail()) });
    open();
    await vi.waitFor(() => expect(skinOf()).toEqual(GO_SKIN_13));
  });

  it("a_variant_without_its_own_skin_gets_the_games", async () => {
    signedIn();
    stubFetch({
      [STATE]: () => json(variantMatch({ slug: "go", variant_label: "9x9" })),
      [GO]: () => json(goDetail()),
    });
    open();

    await vi.waitFor(() => expect(skinOf()).toEqual(goDetail().page.skin));
  });

  it("a_match_whose_game_cannot_be_loaded_still_renders", async () => {
    signedIn();
    stubFetch({
      [STATE]: () => json(variantMatch()),
      [GO]: () => json({ error: "boom" }, 500),
    });
    open();

    expect(await screen.findByLabelText("WHITE ROOK at 0,0")).toBeTruthy();
    expect(title()).toContain("go-13");
    expect(backLink()?.getAttribute("href")).toBe("/games/go");
  });
});
