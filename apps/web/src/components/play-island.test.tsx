import { fireEvent, render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { BoardView } from "../api/types";
import { boardView, choice } from "../test/fixtures";
import { PlayIsland } from "./play-island";

const shogi = (overrides: Partial<BoardView> = {}) =>
  boardView({
    players: ["SENTE", "GOTE"],
    turn: "SENTE",
    hands: true,
    hand: { SENTE: { BISHOP: 1, PAWN: 2 }, GOTE: { LANCE: 1 } },
    choice: choice({
      places: [{ name: "0,0", for: "PIECE" }],
      hand: ["BISHOP"],
      actions: [
        { name: "MOVE", plays: false },
        { name: "DROP", plays: false },
      ],
    }),
    ...overrides,
  });

function island(
  view: BoardView,
  props: Partial<Parameters<typeof PlayIsland>[0]> = {},
) {
  const onChoose = vi.fn();
  const onCancel = vi.fn();
  render(() => (
    <PlayIsland
      view={view}
      skin={null}
      seat="SENTE"
      interactive={true}
      onChoose={onChoose}
      onCancel={onCancel}
      {...props}
    />
  ));
  return { onChoose, onCancel };
}

afterEach(() => vi.restoreAllMocks());

describe("PlayIsland", () => {
  it("asks_for_the_promotion_choice_and_reports_it", () => {
    const { onChoose } = island(boardView({ choice: choice({ codes: ["QUEEN", "ROOK"] }) }), { seat: "WHITE" });

    expect(screen.getByText(/action required/i)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Promote to ROOK" }));

    expect(onChoose).toHaveBeenCalledWith({ code: "ROOK" });
    expect(screen.getByRole("button", { name: "Promote to QUEEN" })).toBeTruthy();
  });

  it("gives_a_spectator_no_promotion_buttons", () => {
    island(boardView({ choice: choice({ codes: ["QUEEN", "ROOK"] }) }), { seat: null, interactive: false });

    expect(screen.queryByRole("button", { name: /promote to/i })).toBeNull();
    expect(screen.queryByText(/action required/i)).toBeNull();
  });

  it("shows_your_hand_with_counts_and_lets_you_pick_a_piece", () => {
    const { onChoose } = island(shogi());

    expect(screen.getByText(/your hand/i)).toBeTruthy();
    const bishop = screen.getByRole("button", { name: "Drop BISHOP" });
    expect(bishop.textContent).toContain("×1");
    expect(screen.getByRole("button", { name: "Drop PAWN" }).textContent).toContain("×2");

    fireEvent.click(bishop);

    expect(onChoose).toHaveBeenCalledWith({ hand: "BISHOP" });
  });

  it("picking_the_chosen_piece_again_puts_it_back", () => {
    const { onChoose, onCancel } = island(
      shogi({ choice: choice({ chosen: [{ hand: "BISHOP" }], places: [{ name: "1,1", for: "PLACE" }] }) }),
    );

    const bishop = screen.getByRole("button", { name: "Drop BISHOP" });
    expect(bishop.getAttribute("aria-pressed")).toBe("true");
    fireEvent.click(bishop);

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onChoose).not.toHaveBeenCalled();
  });

  it("a_piece_with_nowhere_to_go_cannot_be_picked", () => {
    const { onChoose } = island(shogi());

    const pawn = screen.getByRole("button", { name: "Drop PAWN" }) as HTMLButtonElement;
    expect(pawn.disabled).toBe(true);
    fireEvent.click(pawn);

    expect(onChoose).not.toHaveBeenCalled();
  });

  it("shows_the_opponents_hand_but_does_not_let_you_touch_it", () => {
    island(shogi());

    expect(screen.getByText(/gote's hand/i)).toBeTruthy();
    expect(screen.getByText(/lance/i).textContent).toContain("×1");
    expect(screen.queryByRole("button", { name: "Drop LANCE" })).toBeNull();
  });

  it("says_so_when_your_own_hand_is_empty_but_leaves_out_an_empty_opponent_hand", () => {
    island(shogi({ hand: { SENTE: {}, GOTE: {} }, choice: choice() }));

    expect(screen.getAllByText(/nothing in hand/i)).toHaveLength(1);
    expect(screen.getByText(/your hand/i)).toBeTruthy();
    expect(screen.queryByText(/gote's hand/i)).toBeNull();
  });

  it("shows_an_opponent_hand_as_soon_as_it_holds_something", () => {
    island(shogi({ hand: { SENTE: {}, GOTE: { PAWN: 1 } }, choice: choice() }));

    expect(screen.getByText(/gote's hand/i)).toBeTruthy();
  });

  it("a_spectator_is_shown_no_hand_while_both_are_empty", () => {
    island(shogi({ hand: { SENTE: {}, GOTE: {} }, choice: choice() }), { seat: null, interactive: false });

    expect(screen.queryByText(/nothing in hand/i)).toBeNull();
    expect(screen.queryByText(/sente's hand/i)).toBeNull();
  });

  it("rings_the_island_only_while_you_can_act", () => {
    island(shogi());
    expect(screen.getByLabelText("What to do next").classList.contains("island-quiet")).toBe(false);
  });

  it("goes_quiet_while_you_wait", () => {
    island(shogi({ turn: "GOTE", choice: choice() }), { interactive: false });
    expect(screen.getByLabelText("What to do next").classList.contains("island-quiet")).toBe(true);
  });

  it("a_spectator_sees_both_hands_without_buttons", () => {
    island(shogi(), { seat: null, interactive: false });

    expect(screen.getByText(/sente's hand/i)).toBeTruthy();
    expect(screen.getByText(/gote's hand/i)).toBeTruthy();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });

  it("the_hand_cannot_be_used_when_it_is_not_your_turn", () => {
    island(shogi({ turn: "GOTE", choice: choice() }), { interactive: false });

    expect(screen.queryAllByRole("button")).toHaveLength(0);
    expect(screen.getByText(/your hand/i)).toBeTruthy();
  });

  it("a_game_without_a_hand_shows_no_hand", () => {
    island(boardView(), { seat: "WHITE" });

    expect(screen.queryByText(/hand/i)).toBeNull();
  });

  it.each([
    ["nothing_is_picked", {}, /select a piece/i],
    ["a_piece_is_selected", { choice: choice({ start: "0,0", chosen: [{ place: "0,0" }] }) }, /choose a highlighted square/i],
    ["a_hand_game_idle", "hand", /or drop one you hold/i],
  ])("hints_what_to_do_when_%s", (_name, overrides, hint) => {
    if (overrides === "hand") island(shogi(), { seat: "SENTE" });
    else island(boardView(overrides as Partial<BoardView>), { seat: "WHITE" });

    expect(screen.getByText(hint)).toBeTruthy();
  });

  it("hints_at_the_chosen_hand_piece", () => {
    island(shogi({ choice: choice({ chosen: [{ hand: "BISHOP" }], places: [{ name: "1,1", for: "PLACE" }] }) }));

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

describe("PlayIsland with a place for a stone", () => {
  const go = (overrides: Partial<BoardView> = {}) =>
    boardView({
      players: ["BLACK", "WHITE"],
      turn: "BLACK",
      choice: choice({
        places: [{ name: "1,1", for: "PLACE" }],
        actions: [
          { name: "STONE", plays: false },
          { name: "PASS", plays: true },
        ],
      }),
      score: { BLACK: 12, WHITE: 17.5 },
      ...overrides,
    });

  it("offers_a_pass_to_the_player_to_move_and_reports_it", () => {
    const { onChoose } = island(go(), { seat: "BLACK" });

    fireEvent.click(screen.getByRole("button", { name: "Pass" }));

    expect(onChoose).toHaveBeenCalledTimes(1);
    expect(onChoose).toHaveBeenCalledWith({ action: "PASS" });
  });

  it("offers_no_pass_to_a_spectator_or_out_of_turn", () => {
    island(go(), { seat: null, interactive: false });

    expect(screen.queryByRole("button", { name: "Pass" })).toBeNull();
  });

  it("offers_no_pass_in_a_game_that_does_not_allow_it", () => {
    island(go({ choice: choice({ places: [{ name: "1,1", for: "PLACE" }] }) }), { seat: "BLACK" });

    expect(screen.queryByRole("button", { name: "Pass" })).toBeNull();
  });

  it("a_single_supply_action_has_no_button_at_the_start_of_a_turn", () => {
    island(
      go({
        choice: choice({
          places: [{ name: "1,1", for: "PLACE" }],
          actions: [
            { name: "STONE", plays: false },
            { name: "PASS", plays: true },
          ],
        }),
      }),
      { seat: "BLACK" },
    );

    expect(screen.queryByRole("button", { name: "Stone" })).toBeNull();
    expect(screen.getByRole("button", { name: "Pass" })).toBeTruthy();
  });

  it("with_two_supply_pieces_one_has_to_be_chosen_first", () => {
    const { onChoose } = island(
      go({
        choice: choice({
          places: [{ name: "1,1", for: "PLACE" }],
          actions: [
            { name: "STONE", plays: false },
            { name: "DISC", plays: false },
            { name: "PASS", plays: true },
          ],
        }),
      }),
      { seat: "BLACK" },
    );

    fireEvent.click(screen.getByRole("button", { name: "Disc" }));

    expect(onChoose).toHaveBeenCalledWith({ action: "DISC" });
    expect(screen.getByText(/place a piece on an empty point, or pass/i)).toBeTruthy();
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

  it("shows_no_hand_for_a_game_without_one", () => {
    island(go(), { seat: "BLACK" });

    expect(screen.queryByText(/your hand/i)).toBeNull();
  });
});
