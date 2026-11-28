import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { fireEvent, render, screen, waitFor, within } from "@solidjs/testing-library";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SpecCheck } from "../builder/engine";
import { json, stubFetch } from "../test/server";
import { detail, entry } from "../test/fixtures";
import { BuilderPage } from "./builder";

vi.mock("../components/board-loader", () => ({ loadBoardElement: vi.fn().mockResolvedValue(undefined) }));

// The real engine is WebAssembly, built by `make wasm`. This stands in for what it answers.
const engine = vi.hoisted(() => ({ checkSpec: vi.fn() }));
vi.mock("../builder/engine", () => engine);

function reading(text: string): SpecCheck {
  try {
    const spec = JSON.parse(text) as { name?: string };
    if ("broken" in spec) {
      return { ok: false, message: "unknown field `broken`", line: 3, column: 5, path: "board.broken" };
    }
    return { ok: true, name: spec.name ?? "?", players: ["WHITE", "BLACK"] };
  } catch {
    return { ok: false, message: "expected value", line: 1, column: 1, path: null };
  }
}

function show() {
  const history = createMemoryHistory();
  history.set({ value: "/build" });
  render(() => (
    <MemoryRouter history={history}>
      <Route path="/build" component={BuilderPage} />
    </MemoryRouter>
  ));
}

const rulesBox = () => screen.getByLabelText("The game's rules, as JSON") as HTMLTextAreaElement;
const playing = () =>
  JSON.parse(document.querySelector(".builder-play modern-chess-board script")?.textContent ?? "null") as { name?: string } | null;

function type(box: HTMLTextAreaElement, text: string) {
  fireEvent.input(box, { target: { value: text } });
}

beforeEach(() => {
  localStorage.clear();
  engine.checkSpec.mockReset();
  engine.checkSpec.mockImplementation((text: string) => Promise.resolve(reading(text)));
  stubFetch({ "GET /api/game/catalog": () => json([entry({ slug: "chess", name: "Chess" })]) });
});

describe("Workbench", () => {
  it("starts with the smallest game, says it is a game, and plays it", async () => {
    show();

    expect(await screen.findByText("This is a game.")).toBeTruthy();
    expect(screen.getByRole("status").textContent).toContain("MARCH, for WHITE and BLACK.");
    await waitFor(() => expect(playing()?.name).toBe("MARCH"));
  });

  it("says what the engine reports, where, and goes to the line", async () => {
    show();
    await screen.findByText("This is a game.");

    type(rulesBox(), JSON.stringify({ name: "X", broken: 1 }, null, 2));

    expect(await screen.findByText("The engine cannot read this.")).toBeTruthy();
    expect(screen.getByRole("status").textContent).toContain("unknown field `broken`");
    expect(within(screen.getByRole("status")).getByText("board.broken")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Go to line 3" }));
    expect(document.activeElement).toBe(rulesBox());
    expect(rulesBox().selectionStart).toBeGreaterThan(0);
  });

  it("does not start a game the engine cannot read", async () => {
    localStorage.setItem(
      "hubbable.drafts",
      JSON.stringify({ drafts: [{ id: "d", name: "Bad", rules: "{", skin: "", updated: 1 }], current: "d" }),
    );
    show();

    expect(await screen.findByText("The engine cannot read this.")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Run" }) as HTMLButtonElement).disabled).toBe(true);
    expect(document.querySelector("modern-chess-board")).toBeNull();
    expect(screen.getByText("Fix what the engine reports, and your game will start here.")).toBeTruthy();
  });

  it("keeps the game being played while you type, and plays the changes when asked", async () => {
    show();
    await waitFor(() => expect(playing()?.name).toBe("MARCH"));

    type(rulesBox(), JSON.stringify({ name: "STRIDE" }));
    await screen.findByText(/STRIDE, for WHITE and BLACK/);

    expect(playing()?.name).toBe("MARCH");
    expect(screen.getByText("The board is still playing the version you last ran.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Run these changes" }));

    expect(playing()?.name).toBe("STRIDE");
    expect((screen.getByRole("button", { name: "Running" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("tidies the rules into short lines, keeping what fits on one line together", async () => {
    show();
    await screen.findByText("This is a game.");
    type(rulesBox(), '{"name":"X","board":{"dimensions":[3,5]},"pieces":[{"code":"A","name":"a very long name to push it past the width"}]}');

    fireEvent.click(screen.getByRole("button", { name: "Tidy up" }));

    expect(rulesBox().value).toBe(
      [
        "{",
        '  "name": "X",',
        '  "board": { "dimensions": [3, 5] },',
        '  "pieces": [',
        '    { "code": "A", "name": "a very long name to push it past the width" }',
        "  ]",
        "}",
      ].join("\n"),
    );
  });

  it("runs from the keyboard in the editor", async () => {
    show();
    await waitFor(() => expect(playing()?.name).toBe("MARCH"));
    type(rulesBox(), JSON.stringify({ name: "STRIDE" }));
    await screen.findByText(/STRIDE, for WHITE and BLACK/);

    fireEvent.keyDown(rulesBox(), { key: "Enter", ctrlKey: true });

    expect(playing()?.name).toBe("STRIDE");
  });

  it("keeps drafts in the browser and finds them again", async () => {
    show();
    await screen.findByText("This is a game.");
    type(rulesBox(), JSON.stringify({ name: "KEPT" }));

    await waitFor(() => expect(localStorage.getItem("hubbable.drafts")).toContain("KEPT"));
  });

  it("opens the draft that was kept", async () => {
    localStorage.setItem(
      "hubbable.drafts",
      JSON.stringify({ drafts: [{ id: "d", name: "Mine", rules: JSON.stringify({ name: "MINE" }), skin: "", updated: 1 }], current: "d" }),
    );
    show();

    expect(await screen.findByText(/MINE, for WHITE and BLACK/)).toBeTruthy();
    expect((screen.getByLabelText("Draft name") as HTMLInputElement).value).toBe("Mine");
  });

  it("starts a new draft as a copy of a game in the catalogue, with its look", async () => {
    stubFetch({
      "GET /api/game/catalog": () => json([entry({ slug: "chess", name: "Chess" })]),
      "GET /api/game/catalog/chess": () => json(detail({ spec: { name: "Chess", version: 1, seats: [], rules: { name: "CHESS" } } })),
    });
    show();
    await screen.findByText("This is a game.");

    fireEvent.click(screen.getByRole("button", { name: "New draft" }));
    fireEvent.change(await screen.findByDisplayValue("March (the smallest game)"), { target: { value: "chess" } });
    fireEvent.click(screen.getByRole("button", { name: "Create draft" }));

    expect(await screen.findByText(/CHESS, for WHITE and BLACK/)).toBeTruthy();
    expect((screen.getByLabelText("Draft name") as HTMLInputElement).value).toBe("Chess (copy)");
    fireEvent.click(screen.getByRole("tab", { name: "Look" }));
    expect((screen.getByLabelText("The board's look, as JSON") as HTMLTextAreaElement).value).toContain("KING");
  });

  it("says so when the look is not JSON, and will not run until it is", async () => {
    show();
    await waitFor(() => expect(playing()?.name).toBe("MARCH"));
    fireEvent.click(screen.getByRole("tab", { name: "Look" }));

    type(screen.getByLabelText("The board's look, as JSON") as HTMLTextAreaElement, "{ nope");

    expect(await screen.findByText("This look cannot be read.")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Run these changes" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("asks before deleting a draft, and leaves a fresh one when the last goes", async () => {
    show();
    await screen.findByText("This is a game.");
    type(rulesBox(), JSON.stringify({ name: "GONE" }));
    await screen.findByText(/GONE, for/);

    fireEvent.click(screen.getByRole("button", { name: "Delete this draft" }));
    fireEvent.click(screen.getByRole("button", { name: "Keep it" }));
    expect(rulesBox().value).toContain("GONE");

    fireEvent.click(screen.getByRole("button", { name: "Delete this draft" }));
    fireEvent.click(screen.getByRole("button", { name: "Yes, delete it" }));

    expect(await screen.findByText(/MARCH, for/)).toBeTruthy();
    expect(rulesBox().value).not.toContain("GONE");
  });

  it("does not start again a game that never finished starting", async () => {
    localStorage.setItem(
      "hubbable.drafts",
      JSON.stringify({ drafts: [{ id: "d", name: "Hangs", rules: JSON.stringify({ name: "HANGS" }), skin: "", updated: 1 }], current: "d" }),
    );
    localStorage.setItem("hubbable.workbench-run", "d");
    show();

    expect(await screen.findByText(/HANGS, for/)).toBeTruthy();
    expect(screen.getByText(/did not finish starting last time/)).toBeTruthy();
    expect(document.querySelector("modern-chess-board")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Run" }));
    expect(playing()?.name).toBe("HANGS");
  });
});
