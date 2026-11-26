import { fireEvent, render, screen } from "@solidjs/testing-library";
import { describe, expect, it, vi } from "vitest";
import type { BoardView, GameDetail } from "../api/types";
import { boardView, choice } from "../test/fixtures";
import { TablePreview } from "./table-preview";

vi.mock("./board-loader", () => ({ loadBoardElement: vi.fn().mockResolvedValue(undefined) }));

const detail = {
  spec: { rules: {} },
  variants: [],
  preview_variant: null,
  page: { skin: { board_panel: false, labels: { hints: { held: "Choose a card to bid." }, piece_codes: false } } },
} as unknown as GameDetail;

const cards = (overrides: Partial<BoardView> = {}) =>
  boardView({
    players: ["RED", "BLUE"],
    turn: "BLUE",
    bags: [{ name: "HAND", owner: "BLUE", pieces: { C01: 1, C09: 1 } }],
    choice: choice({ held: ["C01", "C09"] }),
    ...overrides,
  });

/** The element the way the engine leaves it: a view to read, and the calls a page may make. */
function engine(view: BoardView) {
  const element = document.querySelector("modern-chess-board") as HTMLElement & Record<string, unknown>;
  Object.defineProperty(element, "view", { get: () => view });
  const calls = { choose: vi.fn(), cancel: vi.fn(), reset: vi.fn() };
  Object.assign(element, calls);
  return calls;
}

describe("TablePreview", () => {
  it("draws_the_table_for_whoever_plays_now_and_hides_the_engines_own_drawing", async () => {
    render(() => <TablePreview detail={detail} />);
    engine(cards());
    document.querySelector("modern-chess-board")?.append(document.createElement("div"));

    expect(await screen.findByText("Your hand")).toBeTruthy();
    expect(screen.getByText("Choose a card to bid.")).toBeTruthy();
    expect(document.querySelector(".live-table-engine")?.getAttribute("aria-hidden")).toBe("true");
  });

  it("passes_a_chosen_card_to_the_engine", async () => {
    render(() => <TablePreview detail={detail} />);
    const calls = engine(cards());
    document.querySelector("modern-chess-board")?.append(document.createElement("div"));

    fireEvent.click(await screen.findByRole("button", { name: "Drop C09" }));

    expect(calls.choose).toHaveBeenCalledWith({ held: "C09" });
  });

  it("starts_the_game_again_on_request", async () => {
    render(() => <TablePreview detail={detail} />);
    const calls = engine(cards());
    document.querySelector("modern-chess-board")?.append(document.createElement("div"));

    fireEvent.click(await screen.findByRole("button", { name: "Start again" }));

    expect(calls.reset).toHaveBeenCalled();
  });

  it("offers_nothing_to_play_once_the_game_is_decided", async () => {
    render(() => <TablePreview detail={detail} />);
    engine(cards({ status: { state: "won", winner: "RED" }, choice: choice() }));
    document.querySelector("modern-chess-board")?.append(document.createElement("div"));

    expect(await screen.findByText("Your hand")).toBeTruthy();
    expect(screen.queryByText("Choose a card to bid.")).toBeNull();
    expect(screen.queryByRole("button", { name: "Drop C09" })).toBeNull();
    expect(screen.getByRole("button", { name: "Start again" })).toBeTruthy();
  });
});
