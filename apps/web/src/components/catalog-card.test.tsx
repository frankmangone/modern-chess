import { Router, Route } from "@solidjs/router";
import { cleanup, render, screen } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { entry } from "../test/fixtures";
import { CatalogCard } from "./catalog-card";

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

  it("links to the game's page", () => {
    const card = show({ entry: entry({ slug: "chess" }) });

    expect(card.getAttribute("href")).toBe("/games/chess");
  });

  it("shows the cover image when the game has one", () => {
    show({ entry: entry({ cover_image: "/covers/chess.png" }) });

    const image = screen.getByRole("img", { name: "Chess" });
    expect(image.getAttribute("src")).toBe("/covers/chess.png");
  });

  it("falls back to a tinted block without a cover image or an accent", () => {
    const card = show({ entry: entry({ cover_image: null, accent: null }) });

    expect(screen.queryByRole("img")).toBeNull();
    expect(card.style.getPropertyValue("--accent")).toBe("#8B4513");
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

  it("loads_the_cover_only_when_it_is_about_to_be_seen", () => {
    show({ entry: entry({ cover_image: "/covers/chess.png" }) });

    expect(screen.getByRole("img", { name: "Chess" }).getAttribute("loading")).toBe("lazy");
  });
});
