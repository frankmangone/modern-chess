import { afterEach, describe, expect, it, vi } from "vitest";
import "@board/modern-chess-board.js";
import { boardView } from "../test/fixtures";
import type { BoardView } from "../api/types";

interface BoardElement extends HTMLElement {
  view: BoardView | null;
  interactive: boolean;
}

async function mount(view: BoardView, interactive = true): Promise<BoardElement> {
  const board = document.createElement("modern-chess-board") as BoardElement;
  board.setAttribute("remote", "");
  document.body.append(board);
  board.view = view;
  board.interactive = interactive;
  await Promise.resolve();
  await Promise.resolve();
  return board;
}

function cell(label: string): HTMLButtonElement {
  const found = document.querySelector<HTMLButtonElement>(`[aria-label="${label}"]`);
  if (found === null) throw new Error(`no cell labelled ${label}`);
  return found;
}

afterEach(() => {
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

describe("<modern-chess-board remote>", () => {
  it("remote_mode_renders_a_given_view_without_loading_wasm", async () => {
    const fetched = vi.spyOn(globalThis, "fetch");
    const instantiate = vi.spyOn(WebAssembly, "instantiate");

    const board = await mount(boardView({ turn: "BLACK" }));

    expect(board.querySelectorAll(".mcb-cell")).toHaveLength(9);
    expect(cell("WHITE ROOK at 0,0")).toBeTruthy();
    expect(board.querySelector(".mcb-status")?.textContent).toContain("BLACK to move");
    expect(board.querySelector(".mcb-reset")).toBeNull();
    expect(fetched).not.toHaveBeenCalled();
    expect(instantiate).not.toHaveBeenCalled();
  });

  it("shows a new view when the property is set again", async () => {
    const board = await mount(boardView());

    board.view = boardView({ pieces: [{ x: 1, y: 1, code: "ROOK", player: "WHITE" }] });

    expect(board.querySelectorAll(".mcb-piece")).toHaveLength(1);
    expect(cell("WHITE ROOK at 1,1")).toBeTruthy();
  });

  it("remote_mode_emits_select_and_move_events", async () => {
    const board = await mount(boardView({ selected: [0, 0], destinations: [[0, 1]] }));
    const heard: [string, unknown][] = [];
    for (const type of ["select", "move", "promote"]) {
      board.addEventListener(type, (event) => heard.push([type, (event as CustomEvent).detail]));
    }

    cell("WHITE ROOK at 0,0").click();
    cell("empty square 0,1").click();

    expect(heard).toEqual([
      ["select", { x: 0, y: 0 }],
      ["move", { x: 0, y: 1 }],
    ]);
  });

  it("emits promote with the chosen piece", async () => {
    const board = await mount(boardView({ promotion: ["QUEEN", "ROOK"] }));
    const heard: unknown[] = [];
    board.addEventListener("promote", (event) => heard.push((event as CustomEvent).detail));

    board.querySelector<HTMLButtonElement>('.mcb-promo-option[title="QUEEN"]')?.click();

    expect(heard).toEqual([{ code: "QUEEN" }]);
  });

  it("does not let a click through when it is not the viewer's turn", async () => {
    const board = await mount(boardView(), false);
    const heard: Event[] = [];
    board.addEventListener("select", (event) => heard.push(event));

    expect(cell("WHITE ROOK at 0,0").disabled).toBe(true);
    cell("WHITE ROOK at 0,0").click();

    expect(heard).toHaveLength(0);
  });

  it("takes properties set before the element was defined", async () => {
    const early = document.createElement("div");
    early.innerHTML = "<modern-chess-board remote></modern-chess-board>";
    const board = early.firstElementChild as BoardElement;
    expect(board.view).toBeNull();

    document.body.append(early);
    board.view = boardView();
    await Promise.resolve();
    await Promise.resolve();

    expect(board.querySelectorAll(".mcb-cell")).toHaveLength(9);
  });
});
