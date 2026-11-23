import { afterEach, describe, expect, it } from "vitest";
import "@board/modern-chess-board.js";
import { boardView, choice } from "../test/fixtures";
import type { BoardView } from "../api/types";

interface BoardElement extends HTMLElement {
  view: BoardView | null;
}

async function mount(view: BoardView): Promise<BoardElement> {
  const board = document.createElement("modern-chess-board") as BoardElement;
  board.setAttribute("remote", "");
  document.body.append(board);
  board.view = view;
  await Promise.resolve();
  await Promise.resolve();
  return board;
}

const sizer = (board: HTMLElement) => board.querySelector(".mcb-status-sizer");

afterEach(() => {
  document.body.replaceChildren();
});

describe("<modern-chess-board> status line", () => {
  it("keeps_room_for_its_longest_line_so_the_board_does_not_move_as_the_text_changes", async () => {
    const board = await mount(boardView());
    const before = sizer(board)?.textContent;

    board.view = boardView({ choice: choice({ start: "0,0", chosen: [{ place: "0,0" }], places: [{ name: "0,1", for: "MOVE" }] }) });

    expect(before).toBe("WHITE to move — pick a highlighted square");
    expect(sizer(board)?.textContent).toBe(before);
    expect(board.querySelector(".mcb-status-text")?.textContent).toBe(
      "WHITE to move — pick a highlighted square",
    );
  });

  it("the_longest_line_is_hidden_from_everyone_but_the_layout", async () => {
    const board = await mount(boardView());

    expect(sizer(board)?.getAttribute("aria-hidden")).toBe("true");
  });

  it("the_line_names_the_player_to_move_in_the_room_it_keeps", async () => {
    const board = await mount(boardView({ turn: "BLACK" }));

    expect(sizer(board)?.textContent).toBe("BLACK to move — pick a highlighted square");
  });
});
