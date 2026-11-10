import { afterEach, describe, expect, it, vi } from "vitest";

// A stand-in for the engine that behaves like the real one after a panic: the call traps, and
// that Board object then refuses every call, while a new Board works.
vi.mock("@board/pkg/modern_chess_wasm.js", () => {
  class Board {
    poisoned = false;
    withHand: boolean;
    withSupply: boolean;
    passed = 0;
    handCount = 1;
    turn = "WHITE";
    selected: [number, number] | null = null;
    pieces = [
      { x: 0, y: 0, code: "ROOK", player: "WHITE" },
      { x: 2, y: 2, code: "ROOK", player: "BLACK" },
    ];

    constructor(spec: string) {
      this.withHand = spec.includes("with a hand");
      this.withSupply = spec.includes("with a supply");
    }

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
        hands: this.withHand,
        hand: this.withHand ? { WHITE: this.handCount > 0 ? { PAWN: this.handCount } : {}, BLACK: {} } : {},
        drops: this.withSupply
          ? { STONE: [[1, 1]] }
          : this.withHand && this.turn === "WHITE" && this.handCount > 0
            ? { PAWN: [[1, 1], [1, 2]] }
            : {},
        supply: this.withSupply ? ["STONE"] : [],
        can_pass: this.withSupply,
        passes: this.passed,
        score: this.withSupply ? { WHITE: 1, BLACK: 7.5 } : {},
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

    drop_piece(code: string, x: number, y: number) {
      this.guard();
      this.pieces.push({ x, y, code, player: this.turn });
      this.handCount -= 1;
      this.turn = this.turn === "WHITE" ? "BLACK" : "WHITE";
    }

    pass() {
      this.guard();
      this.passed += 1;
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

async function mount(spec = "{}"): Promise<BoardElement> {
  const board = document.createElement("modern-chess-board") as BoardElement;
  board.innerHTML = `<script type="application/json">${spec}</script>`;
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

describe("<modern-chess-board> with a hand", () => {
  const WITH_A_HAND = '{"name": "with a hand"}';
  const dests = (board: HTMLElement) =>
    Array.from(board.querySelectorAll(".mcb-dest")).map((c) => c.getAttribute("aria-label"));
  const chip = (board: HTMLElement) => board.querySelector<HTMLButtonElement>(".mcb-hand-piece");

  it("a_game_with_a_hand_is_playable_and_shows_the_hand_of_the_player_to_move", async () => {
    const board = await mount(WITH_A_HAND);

    expect(board.querySelector(".mcb-note")).toBeNull();
    expect(document.querySelector<HTMLButtonElement>('[aria-label="WHITE ROOK at 0,0"]')?.disabled).toBe(false);
    expect(chip(board)?.getAttribute("aria-label")).toBe("PAWN in hand, 1");
  });

  it("choosing_a_hand_piece_lights_its_squares_and_clicking_one_drops_it", async () => {
    const board = await mount(WITH_A_HAND);

    chip(board)?.click();
    expect(dests(board)).toEqual(["empty square 1,2", "empty square 1,1"]);
    click("empty square 1,1");

    expect(board.view.turn).toBe("BLACK");
    expect(board.view.pieces.some((p) => p.x === 1 && p.y === 1 && p.player === "WHITE")).toBe(true);
    expect(chip(board)).toBeNull();
    expect(dests(board)).toEqual([]);
  });

  it("choosing_the_same_hand_piece_again_puts_it_back", async () => {
    const board = await mount(WITH_A_HAND);

    chip(board)?.click();
    chip(board)?.click();

    expect(dests(board)).toEqual([]);
  });

  it("picking_up_a_piece_on_the_board_drops_the_hand_choice", async () => {
    const board = await mount(WITH_A_HAND);
    chip(board)?.click();

    click("WHITE ROOK at 0,0");

    expect(board.view.selected).toEqual([0, 0]);
    expect(dests(board)).toEqual(["empty square 0,1"]);
  });

  it("restores_a_dropped_piece_when_the_engine_crashes_later", async () => {
    const board = await mount(WITH_A_HAND);
    chip(board)?.click();
    click("empty square 1,1");
    click("BLACK ROOK at 2,2");

    click("empty square 2,0");

    expect(board.querySelector(".mcb-message")?.textContent).toContain("undone");
    expect(board.view.pieces.some((p) => p.x === 1 && p.y === 1 && p.player === "WHITE")).toBe(true);
  });
});

describe("<modern-chess-board> with a supply", () => {
  const WITH_A_SUPPLY = '{"name": "with a supply"}';

  it("shows_a_pass_button_and_the_score_and_passing_hands_over_the_turn", async () => {
    const board = await mount(WITH_A_SUPPLY);

    expect(board.querySelector(".mcb-score")?.textContent).toContain("BLACK 7.5");
    board.querySelector<HTMLButtonElement>(".mcb-pass")?.click();

    expect(board.view.turn).toBe("BLACK");
  });

  it("a_stone_is_placed_by_clicking_an_empty_square_when_the_engine_offers_it", async () => {
    const board = await mount(WITH_A_SUPPLY);
    click("empty square 1,1");

    expect(board.view.pieces.length).toBe(3);
  });

  it("a_game_without_a_supply_has_no_pass_button_or_score", async () => {
    const board = await mount();

    expect(board.querySelector(".mcb-pass")).toBeNull();
    expect(board.querySelector(".mcb-score")).toBeNull();
  });
});
