import { Router, Route } from "@solidjs/router";
import { render, screen } from "@solidjs/testing-library";
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
    const card = show({ entry: entry({ accent: "#8a5a2b", tagline: "The classic duel of kings." }), variant: "small" });

    expect(card.style.getPropertyValue("--accent")).toBe("#8a5a2b");
    expect(card.textContent).toContain("The classic duel of kings.");
    expect(card.textContent).toContain("Chess");
  });

  it("links to the game's page", () => {
    const card = show({ entry: entry({ slug: "chess" }), variant: "featured" });

    expect(card.getAttribute("href")).toBe("/games/chess");
  });

  it("shows the cover image when the game has one", () => {
    show({ entry: entry({ cover_image: "/covers/chess.png" }), variant: "wide" });

    const image = screen.getByRole("img", { name: "Chess" });
    expect(image.getAttribute("src")).toBe("/covers/chess.png");
  });

  it("falls back to a tinted block without a cover image or an accent", () => {
    const card = show({ entry: entry({ cover_image: null, accent: null }), variant: "small" });

    expect(screen.queryByRole("img")).toBeNull();
    expect(card.style.getPropertyValue("--accent")).toBe("#8B4513");
  });
});
