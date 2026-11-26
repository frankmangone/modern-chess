import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import { detail } from "../test/fixtures";
import { json, stubFetch } from "../test/server";
import { RulesPage } from "./rules";

vi.mock("../components/board-loader", () => ({ loadBoardElement: vi.fn().mockResolvedValue(undefined) }));

const GAME = "GET /api/game/catalog/chess";

function open() {
  const history = createMemoryHistory();
  history.set({ value: "/games/chess/rules" });
  render(() => (
    <MemoryRouter history={history}>
      <Route path="/games/:slug/rules" component={RulesPage} />
    </MemoryRouter>
  ));
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("RulesPage", () => {
  it("shows_the_rules_with_the_games_live_board_beside_them", async () => {
    stubFetch({ [GAME]: () => json(detail()) });
    open();

    expect((await screen.findByRole("heading", { level: 1 })).textContent).toBe("How to play Chess");
    expect(document.querySelector(".rules-text")?.textContent).toContain("Capture the king.");
    expect(document.querySelector(".game-hero-aside modern-chess-board")).not.toBeNull();
  });

  it("leads_back_to_the_game_and_on_to_its_lobbies", async () => {
    stubFetch({ [GAME]: () => json(detail()) });
    open();

    const crumbs = await screen.findByRole("navigation", { name: "Breadcrumb" });

    expect([...crumbs.querySelectorAll("a")].map((a) => [a.textContent, a.getAttribute("href")])).toEqual([
      ["Games", "/"],
      ["Chess", "/games/chess"],
    ]);
    expect(screen.getByRole("link", { name: "Play a friend" }).getAttribute("href")).toBe("/games/chess#play");
    expect(screen.getByRole("link", { name: "Back to Chess" }).getAttribute("href")).toBe("/games/chess");
  });

  it("says_so_when_the_rules_have_not_been_written", async () => {
    stubFetch({ [GAME]: () => json(detail({ page: { ...detail().page, sections: [{ type: "preview" }] } })) });
    open();

    expect(await screen.findByText(/have not been written yet/i)).toBeTruthy();
  });

  it("shows_no_board_for_a_game_without_a_preview", async () => {
    stubFetch({ [GAME]: () => json(detail({ page: { ...detail().page, sections: [{ type: "how_to_play", markdown: "Rules." }] } })) });
    open();

    await screen.findByRole("heading", { level: 1 });

    expect(document.querySelector("modern-chess-board")).toBeNull();
  });

  it("names_the_page_after_the_rules", async () => {
    stubFetch({ [GAME]: () => json(detail()) });
    open();

    await screen.findByRole("heading", { level: 1 });

    expect(document.title).toBe("How to play Chess · BoardHub");
  });

  it("says_so_when_the_game_does_not_exist", async () => {
    stubFetch({ [GAME]: () => json({ error: "not found" }, 404) });
    open();

    expect((await screen.findByRole("alert")).textContent).toMatch(/couldn't find this game/i);
  });
});
