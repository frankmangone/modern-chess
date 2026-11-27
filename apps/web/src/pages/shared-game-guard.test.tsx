import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
import { fireEvent, render, screen, waitFor } from "@solidjs/testing-library";
import { beforeEach, expect, it, vi } from "vitest";
import { json, stubFetch } from "../test/server";
import { SharedGamePage } from "./shared-game";

const engine = vi.hoisted(() => ({ checkSpec: vi.fn() }));
vi.mock("../builder/engine", () => engine);
// The board draws only in a browser with the board element; this stands in for it, and says it has
// drawn when told to, which is what the page waits for.
vi.mock("../builder/playground", () => ({
  Playground: (props: { onAlive: () => void }) => (
    <button type="button" onClick={props.onAlive}>
      the board draws
    </button>
  ),
}));

const SECRET = "SECRETSECRETSECRETSECRE";
const KEY = "hubbable.shared-run";

beforeEach(() => {
  localStorage.clear();
  engine.checkSpec.mockResolvedValue({ ok: true, name: "STRIDE", players: ["A", "B"] });
  stubFetch({ [`GET /api/game/shared/${SECRET}`]: () => json({ name: "Stride", rules: "{}", skin: "" }) });
  const history = createMemoryHistory();
  history.set({ value: `/build/shared/${SECRET}` });
  render(() => (
    <MemoryRouter history={history}>
      <Route path="/build/shared/:token" component={SharedGamePage} />
    </MemoryRouter>
  ));
});

it("holds the guard while the game starts, and lets it go once the board has drawn", async () => {
  const draws = await screen.findByRole("button", { name: "the board draws" });
  expect(localStorage.getItem(KEY)).toBe(SECRET);

  fireEvent.click(draws);

  await waitFor(() => expect(localStorage.getItem(KEY)).toBeNull());
});
