import { Router, Route } from "@solidjs/router";
import { cleanup, render, screen } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import { boardView, entry } from "../test/fixtures";
import { CatalogCard } from "./catalog-card";

vi.mock("./board-loader", () => ({ loadBoardElement: vi.fn().mockResolvedValue(undefined) }));

function show(props: Parameters<typeof CatalogCard>[0]) {
  render(() => (
    <Router>
      <Route path="/" component={() => <CatalogCard {...props} />} />
    </Router>
  ));
  return screen.getByTestId("catalog-card");
}

describe("CatalogCard", () => {
  it("catalog_card_uses_the_games_accent_and_tagline", () => {
    const card = show({ entry: entry({ accent: "#8a5a2b", tagline: "The classic duel of kings." }) });

    expect(card.style.getPropertyValue("--accent")).toBe("#8a5a2b");
    expect(card.textContent).toContain("The classic duel of kings.");
    expect(card.textContent).toContain("Chess");
  });

  it("its name links to the game's page", () => {
    show({ entry: entry({ slug: "chess", name: "Chess" }) });

    expect(screen.getByRole("link", { name: "Chess" }).getAttribute("href")).toBe("/games/chess");
  });

  it("shows the game's board as it starts, drawn in the game's skin with nothing to click", () => {
    const skin = { squares: { light: "#fff", dark: "#000" } };
    const view = boardView();
    show({ entry: entry({ board: view, skin }) });

    const board = screen.getByTestId("game-board");
    const element = board.querySelector("modern-chess-board") as HTMLElement & {
      view: unknown;
      interactive: boolean;
    };
    expect(board.getAttribute("aria-hidden")).toBe("true");
    expect(element.hasAttribute("remote")).toBe(true);
    expect(element.view).toEqual(view);
    expect(element.interactive).toBe(false);
    expect(decodeURIComponent(element.getAttribute("skin") ?? "")).toContain('"#fff"');
  });

  it("shows the tinted block for a game whose skin turns the board off, though it has a view", () => {
    const card = show({ entry: entry({ board: boardView(), skin: { board_panel: false } }) });

    expect(screen.queryByTestId("game-board")).toBeNull();
    expect(card.querySelector(".game-card-tint")).not.toBeNull();
  });

  it("falls back to a tinted block without a board or an accent", () => {
    const card = show({ entry: entry({ board: null, accent: null }) });

    expect(screen.queryByTestId("game-board")).toBeNull();
    expect(card.querySelector(".game-card-tint")).not.toBeNull();
    expect(card.style.getPropertyValue("--accent")).toBe("#8B4513");
  });

  it("links to the game's variants on its page, by how many there are", () => {
    show({ entry: entry({ slug: "chess" }), variantCount: 7 });

    const link = screen.getByRole("link", { name: /7 variants/ });
    expect(link.getAttribute("href")).toBe("/games/chess#variants");
  });

  it("says variant, not variants, for one", () => {
    show({ entry: entry(), variantCount: 1 });

    expect(screen.getByRole("link", { name: /^1 variant\b/ })).toBeTruthy();
  });

  it("has no variants link for a game without any", () => {
    show({ entry: entry() });

    expect(screen.queryByRole("link", { name: /variant/ })).toBeNull();
  });

  it("names_one_kind_of_game_and_leaves_out_the_player_count", () => {
    const card = show({ entry: entry({ tags: ["two players", "classic", "strategy"] }) });

    expect(card.textContent).toContain("classic");
    expect(card.textContent).not.toContain("two players");
    expect(card.textContent).not.toContain("strategy");
  });

  it("leaves_the_kind_out_when_the_list_it_sits_in_is_all_of_one", () => {
    const card = show({ entry: entry({ tags: ["strategy"] }), showKind: false });

    expect(card.textContent).not.toContain("strategy");
  });

  it("only_the_lead_has_the_way_in_on_it", () => {
    expect(show({ entry: entry() }).textContent).not.toContain("Play now");
    cleanup();
    expect(show({ entry: entry(), size: "side" }).textContent).not.toContain("Play now");
    cleanup();

    const lead = show({ entry: entry(), size: "lead" });
    expect(lead.textContent).toContain("Play now");
    expect(lead.classList.contains("game-card-lead")).toBe(true);
  });

  it("a_side_card_is_marked_for_its_own_layout", () => {
    expect(show({ entry: entry(), size: "side" }).classList.contains("game-card-side")).toBe(true);
  });

  it("says_whether_the_game_is_pinned_without_a_badge", () => {
    const featured = show({ entry: entry({ featured: true }) });

    expect(featured.getAttribute("data-featured")).toBe("true");
    expect(featured.textContent).not.toMatch(/featured/i);
  });
});
