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

  it("falls back to a tinted block without a board or an accent", () => {
    const card = show({ entry: entry({ board: null, accent: null }) });

    expect(screen.queryByTestId("game-board")).toBeNull();
    expect(card.querySelector(".game-card-tint")).not.toBeNull();
    expect(card.style.getPropertyValue("--accent")).toBe("#8B4513");
  });

  it("links each of the game's variants by name", () => {
    show({
      entry: entry(),
      variantGames: [
        entry({ slug: "atomic", name: "Atomic" }),
        entry({ slug: "crazyhouse", name: "Crazyhouse" }),
      ],
    });

    const variants = screen.getByRole("navigation", { name: "Variants of Chess" });
    const links = [...variants.querySelectorAll("a")].map((a) => [a.textContent, a.getAttribute("href")]);
    expect(links).toEqual([
      ["Atomic", "/games/atomic"],
      ["Crazyhouse", "/games/crazyhouse"],
    ]);
  });

  it("lists no variants for a game without any", () => {
    show({ entry: entry() });

    expect(screen.queryByRole("navigation")).toBeNull();
  });

  it("only_a_featured_game_carries_the_featured_badge", () => {
    const plain = show({ entry: entry({ featured: false }) });
    expect(plain.textContent).not.toMatch(/featured/i);
    expect(plain.getAttribute("data-featured")).toBe("false");
    cleanup();

    const featured = show({ entry: entry({ featured: true }) });
    expect(featured.textContent).toMatch(/featured/i);
    expect(featured.getAttribute("data-featured")).toBe("true");
  });

  it("every_game_gets_the_same_card_with_its_tags_and_a_play_button", () => {
    const card = show({ entry: entry({ featured: false, tags: ["classic", "two players"] }) });

    expect(card.textContent).toContain("classic");
    expect(card.textContent).toContain("two players");
    expect(card.textContent).toContain("Play now");
  });
});
