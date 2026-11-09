import { fireEvent, render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { BoardView } from "../api/types";
import { boardView } from "../test/fixtures";
import { PlayIsland } from "./play-island";

const shogi = (overrides: Partial<BoardView> = {}) =>
  boardView({
    players: ["SENTE", "GOTE"],
    turn: "SENTE",
    hands: true,
    hand: { SENTE: { BISHOP: 1, PAWN: 2 }, GOTE: { LANCE: 1 } },
    drops: { BISHOP: [[1, 1]], PAWN: [] },
    ...overrides,
  });

function island(
  view: BoardView,
  props: Partial<Parameters<typeof PlayIsland>[0]> = {},
) {
  const onPickDrop = vi.fn();
  const onPromote = vi.fn();
  render(() => (
    <PlayIsland
      view={view}
      skin={null}
      seat="SENTE"
      interactive={true}
      dropCode={null}
      onPickDrop={onPickDrop}
      onPromote={onPromote}
      {...props}
    />
  ));
  return { onPickDrop, onPromote };
}

afterEach(() => vi.restoreAllMocks());

describe("PlayIsland", () => {
  it("asks_for_the_promotion_choice_and_reports_it", () => {
    const { onPromote } = island(boardView({ promotion: ["QUEEN", "ROOK"] }), { seat: "WHITE" });

    expect(screen.getByText(/action required/i)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Promote to ROOK" }));

    expect(onPromote).toHaveBeenCalledWith("ROOK");
    expect(screen.getByRole("button", { name: "Promote to QUEEN" })).toBeTruthy();
  });

  it("gives_a_spectator_no_promotion_buttons", () => {
    island(boardView({ promotion: ["QUEEN", "ROOK"] }), { seat: null, interactive: false });

    expect(screen.queryByRole("button", { name: /promote to/i })).toBeNull();
    expect(screen.queryByText(/action required/i)).toBeNull();
  });

  it("shows_your_hand_with_counts_and_lets_you_pick_a_piece", () => {
    const { onPickDrop } = island(shogi());

    expect(screen.getByText(/your hand/i)).toBeTruthy();
    const bishop = screen.getByRole("button", { name: "Drop BISHOP" });
    expect(bishop.textContent).toContain("×1");
    expect(screen.getByRole("button", { name: "Drop PAWN" }).textContent).toContain("×2");

    fireEvent.click(bishop);

    expect(onPickDrop).toHaveBeenCalledWith("BISHOP");
  });

  it("picking_the_chosen_piece_again_puts_it_back", () => {
    const { onPickDrop } = island(shogi(), { dropCode: "BISHOP" });

    const bishop = screen.getByRole("button", { name: "Drop BISHOP" });
    expect(bishop.getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(bishop);

    expect(onPickDrop).toHaveBeenCalledWith(null);
  });

  it("a_piece_with_nowhere_to_go_cannot_be_picked", () => {
    const { onPickDrop } = island(shogi());

    const pawn = screen.getByRole("button", { name: "Drop PAWN" }) as HTMLButtonElement;
    expect(pawn.disabled).toBe(true);
    fireEvent.click(pawn);

    expect(onPickDrop).not.toHaveBeenCalled();
  });

  it("shows_the_opponents_hand_but_does_not_let_you_touch_it", () => {
    island(shogi());

    expect(screen.getByText(/gote's hand/i)).toBeTruthy();
    expect(screen.getByText(/lance/i).textContent).toContain("×1");
    expect(screen.queryByRole("button", { name: "Drop LANCE" })).toBeNull();
  });

  it("says_so_when_a_hand_is_empty", () => {
    island(shogi({ hand: { SENTE: {}, GOTE: {} }, drops: {} }));

    expect(screen.getAllByText(/nothing in hand/i)).toHaveLength(2);
  });

  it("a_spectator_sees_both_hands_without_buttons", () => {
    island(shogi(), { seat: null, interactive: false });

    expect(screen.getByText(/sente's hand/i)).toBeTruthy();
    expect(screen.getByText(/gote's hand/i)).toBeTruthy();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });

  it("the_hand_cannot_be_used_when_it_is_not_your_turn", () => {
    island(shogi({ turn: "GOTE", drops: {} }), { interactive: false });

    expect(screen.queryAllByRole("button")).toHaveLength(0);
    expect(screen.getByText(/your hand/i)).toBeTruthy();
  });

  it("a_game_without_a_hand_shows_no_hand", () => {
    island(boardView(), { seat: "WHITE" });

    expect(screen.queryByText(/hand/i)).toBeNull();
  });

  it.each([
    ["nothing_is_picked", {}, /select a piece/i],
    ["a_piece_is_selected", { selected: [0, 0] as [number, number] }, /choose a highlighted square/i],
    ["a_hand_game_idle", "hand", /or drop one you hold/i],
  ])("hints_what_to_do_when_%s", (_name, overrides, hint) => {
    if (overrides === "hand") island(shogi(), { seat: "SENTE" });
    else island(boardView(overrides as Partial<BoardView>), { seat: "WHITE" });

    expect(screen.getByText(hint)).toBeTruthy();
  });

  it("hints_at_the_chosen_hand_piece", () => {
    island(shogi(), { dropCode: "BISHOP" });

    expect(screen.getByText(/bishop selected/i)).toBeTruthy();
  });

  it("says_who_it_is_waiting_for_on_the_other_players_turn", () => {
    island(boardView({ turn: "BLACK" }), { seat: "WHITE", interactive: false });

    expect(screen.getByText(/waiting for black/i)).toBeTruthy();
  });

  it("draws_hand_pieces_with_the_games_skin", () => {
    island(shogi(), { skin: { pieces: { BISHOP: "角" } } });

    expect(screen.getByRole("button", { name: "Drop BISHOP" }).textContent).toContain("角");
  });
});
