import { Router, Route } from "@solidjs/router";
import { render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import { FOUR_GAMES, entry } from "../test/fixtures";
import { HomePage } from "./home";

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
});
