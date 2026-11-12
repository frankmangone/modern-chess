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
  const onPass = vi.fn();
  render(() => (
    <PlayIsland
      view={view}
      skin={null}
      seat="SENTE"
      interactive={true}
      dropCode={null}
      onPickDrop={onPickDrop}
      onPromote={onPromote}
      onPass={onPass}
      {...props}
    />
  ));
  return { onPickDrop, onPromote, onPass };
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

  it("says_so_when_your_own_hand_is_empty_but_leaves_out_an_empty_opponent_hand", () => {
    island(shogi({ hand: { SENTE: {}, GOTE: {} }, drops: {} }));

    expect(screen.getAllByText(/nothing in hand/i)).toHaveLength(1);
    expect(screen.getByText(/your hand/i)).toBeTruthy();
    expect(screen.queryByText(/gote's hand/i)).toBeNull();
  });

  it("shows_an_opponent_hand_as_soon_as_it_holds_something", () => {
    island(shogi({ hand: { SENTE: {}, GOTE: { PAWN: 1 } }, drops: {} }));

    expect(screen.getByText(/gote's hand/i)).toBeTruthy();
  });

  it("a_spectator_is_shown_no_hand_while_both_are_empty", () => {
    island(shogi({ hand: { SENTE: {}, GOTE: {} }, drops: {} }), { seat: null, interactive: false });

    expect(screen.queryByText(/nothing in hand/i)).toBeNull();
    expect(screen.queryByText(/sente's hand/i)).toBeNull();
  });

  it("rings_the_island_only_while_you_can_act", () => {
    island(shogi());
    expect(screen.getByLabelText("What to do next").classList.contains("island-quiet")).toBe(false);
  });

  it("goes_quiet_while_you_wait", () => {
    island(shogi({ turn: "GOTE", drops: {} }), { interactive: false });
    expect(screen.getByLabelText("What to do next").classList.contains("island-quiet")).toBe(true);
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

describe("PlayIsland with a supply", () => {
  const go = (overrides: Partial<BoardView> = {}) =>
    boardView({
      players: ["BLACK", "WHITE"],
      turn: "BLACK",
      supply: ["STONE"],
      drops: { STONE: [[1, 1]] },
      can_pass: true,
      score: { BLACK: 12, WHITE: 17.5 },
      ...overrides,
    });

  it("offers_a_pass_to_the_player_to_move_and_reports_it", () => {
    const { onPass } = island(go(), { seat: "BLACK" });

    fireEvent.click(screen.getByRole("button", { name: "Pass" }));

    expect(onPass).toHaveBeenCalledTimes(1);
  });

  it("offers_no_pass_to_a_spectator_or_out_of_turn", () => {
    island(go(), { seat: null, interactive: false });

    expect(screen.queryByRole("button", { name: "Pass" })).toBeNull();
  });

  it("offers_no_pass_in_a_game_that_does_not_allow_it", () => {
    island(go({ can_pass: false }), { seat: "BLACK" });

    expect(screen.queryByRole("button", { name: "Pass" })).toBeNull();
  });

  it("shows_the_score_as_if_the_game_ended_now", () => {
    island(go(), { seat: "BLACK" });

    expect(screen.getByText(/if the game ended now/i)).toBeTruthy();
    expect(screen.getByText("17.5")).toBeTruthy();
    expect(screen.getByText("12")).toBeTruthy();
  });

  it("shows_no_score_for_a_game_that_does_not_count", () => {
    island(go({ score: {} }), { seat: "BLACK" });

    expect(screen.queryByText(/if the game ended now/i)).toBeNull();
  });

  it("says_what_to_do_and_notes_a_pass_by_the_other_side", () => {
    island(go({ passes: 1 }), { seat: "BLACK" });

    expect(screen.getByText(/place a stone on an empty point, or pass/i)).toBeTruthy();
    expect(screen.getByText(/the last turn was a pass/i)).toBeTruthy();
  });

  it("shows_no_hand_for_a_supply_game", () => {
    island(go(), { seat: "BLACK" });

    expect(screen.queryByText(/your hand/i)).toBeNull();
  });
});
