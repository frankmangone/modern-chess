import { afterEach, describe, expect, it, vi } from "vitest";

// A stand-in for the engine that behaves like the real one after a panic: the call traps, and
// that Board object then refuses every call, while a new Board works.
vi.mock("@board/pkg/modern_chess_wasm.js", () => {
  class Board {
    poisoned = false;
    turn = "WHITE";
    selected: [number, number] | null = null;
    pieces = [
      { x: 0, y: 0, code: "ROOK", player: "WHITE" },
      { x: 2, y: 2, code: "ROOK", player: "BLACK" },
    ];

    private guard() {
      if (this.poisoned) throw new Error("recursive use of an object detected which would lead to unsafe aliasing in rust");
    }

    view() {
      return JSON.stringify({
        width: 3,
        height: 3,
        disabled: [],
        players: ["WHITE", "BLACK"],
        pieces: this.pieces,
        turn: this.turn,
        selected: this.selected,
        destinations: this.selected ? (this.turn === "WHITE" ? [[0, 1]] : [[2, 1], [2, 0]]) : [],
        promotion: null,
        status: { state: "playing" },
        zones: {},
        hands: false,
        hand: {},
        drops: {},
      });
    }

    select(x: number, y: number) {
      this.guard();
      this.selected = [x, y];
    }

    make_move(x: number, y: number) {
      this.guard();
      if (x === 2 && y === 0) {
        this.poisoned = true;
        throw new WebAssembly.RuntimeError("unreachable");
      }
      const piece = this.pieces.find((p) => this.selected && p.x === this.selected[0] && p.y === this.selected[1]);
      if (!piece) throw new Error("nothing selected");
      piece.x = x;
      piece.y = y;
      this.selected = null;
      this.turn = this.turn === "WHITE" ? "BLACK" : "WHITE";
    }

    promote() {
      this.guard();
    }

    reset() {
      this.guard();
    }
  }
  return { default: () => Promise.resolve(), Board };
});

import "@board/modern-chess-board.js";

interface BoardElement extends HTMLElement {
  view: { turn: string; pieces: { x: number; y: number; player: string }[]; selected: number[] | null };
}

async function mount(): Promise<BoardElement> {
  const board = document.createElement("modern-chess-board") as BoardElement;
  board.innerHTML = '<script type="application/json">{}</script>';
  document.body.append(board);
  await vi.waitFor(() => expect(board.querySelector(".mcb-grid")).not.toBeNull());
  return board;
}

function click(label: string) {
  const found = document.querySelector<HTMLButtonElement>(`[aria-label="${label}"]`);
  if (found === null) throw new Error(`no cell labelled ${label}`);
  found.click();
}

afterEach(() => {
  document.body.replaceChildren();
});

describe("<modern-chess-board> when the engine faults", () => {
  it("restores_the_position_before_the_move_that_crashed_and_keeps_playing", async () => {
    const board = await mount();
    click("WHITE ROOK at 0,0");
    click("empty square 0,1");
    click("BLACK ROOK at 2,2");

    click("empty square 2,0");

    expect(board.querySelector(".mcb-message")?.textContent).toContain("undone");
    expect(board.view.turn).toBe("BLACK");
    expect(board.view.pieces.map((p) => [p.player, p.x, p.y])).toEqual([
      ["WHITE", 0, 1],
      ["BLACK", 2, 2],
    ]);
    expect(board.view.selected).toEqual([2, 2]);

    click("empty square 2,1");

    expect(board.view.turn).toBe("WHITE");
    expect(board.querySelector(".mcb-message")).toBeNull();
  });
});
