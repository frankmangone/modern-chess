import { Router, Route } from "@solidjs/router";
import { render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import { fireEvent } from "@solidjs/testing-library";
import { FOUR_GAMES, entry } from "../test/fixtures";
import { BAND_SIZE, HomePage, band, kinds, listings } from "./home";

vi.mock("../components/board-loader", () => ({ loadBoardElement: vi.fn().mockResolvedValue(undefined) }));

function respondWith(response: Promise<Response>) {
  vi.stubGlobal("fetch", vi.fn().mockReturnValue(response));
}

function catalogue(entries: unknown): Promise<Response> {
  return Promise.resolve(new Response(JSON.stringify(entries), { status: 200 }));
}

function open() {
  render(() => (
    <Router>
      <Route path="/" component={HomePage} />
    </Router>
  ));
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("HomePage", () => {
  it("home_renders_one_card_per_catalog_entry", async () => {
    respondWith(catalogue(FOUR_GAMES));
    open();

    const cards = await screen.findAllByTestId("catalog-card");

    expect(cards.map((card) => card.getAttribute("data-slug"))).toEqual([
      "chess",
      "checkers",
      "ataxx",
      "chaturanga",
    ]);
  });

  it("puts the featured game first even when it is listed later", async () => {
    respondWith(
      catalogue([entry({ slug: "a", name: "A" }), entry({ slug: "b", name: "B", featured: true })]),
    );
    open();

    const cards = await screen.findAllByTestId("catalog-card");

    expect(cards[0].getAttribute("data-slug")).toBe("b");
    expect(cards[0].getAttribute("data-featured")).toBe("true");
    expect(cards[1].getAttribute("data-featured")).toBe("false");
  });

  it("lays out more than four games without dropping any", async () => {
    const many = Array.from({ length: 7 }, (_, i) => entry({ slug: `g${i}`, name: `G${i}` }));
    respondWith(catalogue(many));
    open();

    expect(await screen.findAllByTestId("catalog-card")).toHaveLength(7);
  });

  it("gives each game of its own a card, and links to its variants from that card", async () => {
    respondWith(
      catalogue([
        entry({ slug: "chess", name: "Chess" }),
        entry({ slug: "checkers", name: "Checkers" }),
        entry({ slug: "atomic", name: "Atomic", parent: "chess" }),
        entry({ slug: "crazyhouse", name: "Crazyhouse", parent: "chess", featured: true }),
      ]),
    );
    open();

    const cards = await screen.findAllByTestId("catalog-card");

    expect(cards.map((card) => card.getAttribute("data-slug"))).toEqual(["chess", "checkers"]);
    const variants = screen.getByRole("link", { name: /2 variants/ });
    expect(variants.getAttribute("href")).toBe("/games/chess#variants");
    expect(screen.queryByRole("link", { name: /variant/, hidden: false })).toBe(variants);
  });

  it("gives a variant whose parent is not listed a card of its own", async () => {
    respondWith(catalogue([entry({ slug: "atomic", name: "Atomic", parent: "chess" })]));
    open();

    const cards = await screen.findAllByTestId("catalog-card");

    expect(cards.map((card) => card.getAttribute("data-slug"))).toEqual(["atomic"]);
  });

  it("home_shows_a_readable_message_when_game_api_is_down", async () => {
    respondWith(Promise.reject(new TypeError("Failed to fetch")));
    open();

    const message = await screen.findByRole("alert");

    expect(message.textContent).toMatch(/can't reach the game server/i);
    expect(screen.queryAllByTestId("catalog-card")).toHaveLength(0);
  });

  it("shows the same message when the server answers with an error", async () => {
    respondWith(Promise.resolve(new Response(JSON.stringify({ error: "something went wrong" }), { status: 500 })));
    open();

    expect((await screen.findByRole("alert")).textContent).toMatch(/can't reach the game server/i);
  });

  it("says so when there are no games yet", async () => {
    respondWith(catalogue([]));
    open();

    expect(await screen.findByText(/no games yet/i)).toBeTruthy();
  });

  it("counts the real games in the hero instead of a made-up number", async () => {
    respondWith(catalogue(FOUR_GAMES));
    open();

    await screen.findAllByTestId("catalog-card");

    expect(screen.getByText(/4 games available now/i)).toBeTruthy();
    expect(document.body.textContent).not.toMatch(/60\+|192K|4\.2M|850\+/);
  });

  it("marks_its_main_content_and_names_the_page", async () => {
    respondWith(catalogue(FOUR_GAMES));
    open();

    await screen.findAllByTestId("catalog-card");

    expect(screen.getByRole("main")).toBeTruthy();
    expect(document.title).toBe("BoardHub");
  });

  describe("the featured band", () => {
    const games = [
      entry({ slug: "a", name: "A", tags: ["classic"] }),
      entry({ slug: "b", name: "B", tags: ["modern"], featured: true }),
      entry({ slug: "c", name: "C", tags: ["classic"], featured: true }),
      entry({ slug: "d", name: "D", tags: ["modern"], featured: true }),
      entry({ slug: "e", name: "E", tags: ["modern"], featured: true }),
    ];

    it("holds_the_featured_games_up_to_its_size_and_never_one_nobody_pinned", () => {
      const slugs = band(listings(games)).map((item) => item.game.slug);

      expect(slugs).toEqual(["b", "c", "d"]);
      expect(slugs).toHaveLength(BAND_SIZE);
      expect(band(listings([entry({ slug: "a" }), entry({ slug: "b" })]))).toEqual([]);
    });

    it("leads_with_one_large_card_and_puts_the_next_two_beside_it", async () => {
      respondWith(catalogue(games));
      open();

      await screen.findAllByTestId("catalog-card");
      const cards = screen.getAllByTestId("catalog-card");

      expect(cards.map((card) => card.getAttribute("data-slug"))).toEqual(["b", "c", "d", "e", "a"]);
      expect(cards[0].classList.contains("game-card-lead")).toBe(true);
      expect(cards[1].classList.contains("game-card-side")).toBe(true);
      expect(cards[2].classList.contains("game-card-side")).toBe(true);
      expect(cards[3].classList.contains("game-card-lead")).toBe(false);
      expect(screen.getByRole("heading", { name: "Featured" })).toBeTruthy();
      expect(screen.getByRole("heading", { name: "More to play" })).toBeTruthy();
    });

    it("has_no_band_and_keeps_the_old_title_when_nothing_is_featured", async () => {
      respondWith(catalogue([entry({ slug: "a" }), entry({ slug: "b" })]));
      open();

      await screen.findAllByTestId("catalog-card");

      expect(screen.queryByRole("heading", { name: "Featured" })).toBeNull();
      expect(screen.getByRole("heading", { name: "Play one today" })).toBeTruthy();
      expect(document.querySelector(".game-card-lead")).toBeNull();
    });

    it("lists_each_kind_once_in_the_order_the_page_lists_the_games", () => {
      expect(kinds(listings(games))).toEqual(["modern", "classic"]);
      expect(kinds(listings([entry({ tags: ["two players", "classic"] })]))).toEqual(["classic"]);
    });

    it("shows_only_the_games_of_a_kind_with_the_band_set_aside_until_it_is_cleared", async () => {
      respondWith(catalogue(games));
      open();
      await screen.findAllByTestId("catalog-card");

      fireEvent.click(screen.getByRole("button", { name: "classic" }));

      expect(screen.getAllByTestId("catalog-card").map((card) => card.getAttribute("data-slug"))).toEqual(["c", "a"]);
      expect(screen.getByRole("button", { name: "classic" }).getAttribute("aria-pressed")).toBe("true");
      expect(screen.queryByRole("heading", { name: "Featured" })).toBeNull();
      expect(screen.getByRole("heading", { name: "Classic games" })).toBeTruthy();

      fireEvent.click(screen.getByRole("button", { name: "All" }));

      expect(screen.getAllByTestId("catalog-card")).toHaveLength(5);
      expect(screen.getByRole("heading", { name: "Featured" })).toBeTruthy();
    });
  });
});
