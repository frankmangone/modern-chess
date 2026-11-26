import { fireEvent, render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { BagView, BoardView } from "../api/types";
import { boardView, choice } from "../test/fixtures";
import { PlayIsland } from "./play-island";

const hand = (owner: string, pieces: Record<string, number>): BagView => ({ name: "HAND", owner, pieces });

const shogi = (overrides: Partial<BoardView> = {}) =>
  boardView({
    players: ["SENTE", "GOTE"],
    turn: "SENTE",
    bags: [hand("GOTE", { LANCE: 1 }), hand("SENTE", { BISHOP: 1, PAWN: 2 })],
    choice: choice({
      places: [{ name: "0,0", for: "PIECE" }],
      held: ["BISHOP"],
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
    const { onChoose } = island(
      boardView({ choice: choice({ chosen: [{ place: "0,6" }, { place: "0,7" }], codes: ["QUEEN", "ROOK"] }) }),
      { seat: "WHITE" },
    );

    expect(screen.getByText(/action required/i)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Promote to ROOK" }));

    expect(onChoose).toHaveBeenCalledWith({ code: "ROOK" });
    expect(screen.getByRole("button", { name: "Promote to QUEEN" })).toBeTruthy();
  });

  it("a_choice_between_codes_that_comes_first_is_not_called_a_promotion", () => {
    const { onChoose } = island(boardView({ choice: choice({ codes: ["ROCK", "PAPER"] }) }), { seat: "WHITE" });

    expect(screen.queryByText(/action required/i)).toBeNull();
    expect(screen.queryByText(/promotion/i)).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Choose PAPER" }));

    expect(onChoose).toHaveBeenCalledWith({ code: "PAPER" });
  });

  it("a_decided_game_shows_what_each_side_held_and_no_hint", () => {
    island(
      boardView({
        status: { state: "won", winner: "BLACK" },
        choice: choice(),
        bags: [
          { name: "PICK", owner: "BLACK", pieces: { ROCK: 1 } },
          { name: "PICK", owner: "WHITE", pieces: { SCISSORS: 1 } },
        ],
      }),
      { seat: "WHITE", interactive: false },
    );

    expect(screen.getByText("ROCK ×1")).toBeTruthy();
    expect(screen.getByText("SCISSORS ×1")).toBeTruthy();
    expect(screen.queryByText(/to move|waiting for/i)).toBeNull();
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

    expect(onChoose).toHaveBeenCalledWith({ held: "BISHOP" });
  });

  it("picking_the_chosen_piece_again_puts_it_back", () => {
    const { onChoose, onCancel } = island(
      shogi({ choice: choice({ chosen: [{ held: "BISHOP" }], places: [{ name: "1,1", for: "PLACE" }] }) }),
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
    island(shogi({ bags: [hand("GOTE", {}), hand("SENTE", {})], choice: choice() }));

    expect(screen.getAllByText(/nothing in hand/i)).toHaveLength(1);
    expect(screen.getByText(/your hand/i)).toBeTruthy();
    expect(screen.queryByText(/gote's hand/i)).toBeNull();
  });

  it("shows_an_opponent_hand_as_soon_as_it_holds_something", () => {
    island(shogi({ bags: [hand("GOTE", { PAWN: 1 }), hand("SENTE", {})], choice: choice() }));

    expect(screen.getByText(/gote's hand/i)).toBeTruthy();
  });

  it("a_spectator_is_shown_no_hand_while_both_are_empty", () => {
    island(shogi({ bags: [hand("GOTE", {}), hand("SENTE", {})], choice: choice() }), { seat: null, interactive: false });

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

  it("lists_a_bag_that_is_not_a_hand_under_its_own_name", () => {
    island(
      shogi({
        bags: [
          { name: "BANK", pieces: { STONE: 3 } },
          { name: "STORE", owner: "GOTE", pieces: { SEED: 2 } },
          hand("SENTE", { PAWN: 1 }),
        ],
      }),
    );

    expect(screen.getByText("BANK")).toBeTruthy();
    expect(screen.getByText("STONE ×3")).toBeTruthy();
    expect(screen.getByText("GOTE's STORE")).toBeTruthy();
    expect(screen.getByText("SEED ×2")).toBeTruthy();
    expect(screen.getByText(/your hand/i)).toBeTruthy();
  });

  it("a_held_choice_can_be_answered_from_a_shared_bag", () => {
    const { onChoose } = island(
      shogi({
        bags: [{ name: "BANK", pieces: { STONE: 3 } }, hand("SENTE", {})],
        choice: choice({ held: ["STONE"], actions: [{ name: "TAKE", plays: false }] }),
      }),
    );

    fireEvent.click(screen.getByRole("button", { name: "Choose STONE" }));

    expect(onChoose).toHaveBeenCalledWith({ held: "STONE" });
  });

  it("an_enemy_bag_can_be_chosen_from_when_the_choice_names_its_code", () => {
    const { onChoose } = island(
      shogi({
        bags: [{ name: "STORE", owner: "GOTE", pieces: { SEED: 2 } }, hand("SENTE", {})],
        choice: choice({ held: ["SEED"] }),
      }),
    );

    fireEvent.click(screen.getByRole("button", { name: "Choose SEED" }));

    expect(onChoose).toHaveBeenCalledWith({ held: "SEED" });
  });

  it("a_bag_whose_code_is_not_offered_stays_static", () => {
    island(
      shogi({
        bags: [{ name: "BANK", pieces: { STONE: 3 } }, hand("SENTE", {})],
        choice: choice({ held: ["BISHOP"] }),
      }),
    );

    expect(screen.queryByRole("button", { name: "Choose STONE" })).toBeNull();
  });

  it("only_the_bag_that_would_be_chosen_from_lights_up", () => {
    island(
      shogi({
        bags: [hand("GOTE", { BISHOP: 1 }), hand("SENTE", { BISHOP: 1 })],
        choice: choice({ chosen: [{ held: "BISHOP" }], held: ["BISHOP"], places: [{ name: "1,1", for: "PLACE" }] }),
      }),
    );

    const chips = screen.getAllByRole("button", { name: /BISHOP/ });
    expect(chips.filter((chip) => chip.getAttribute("aria-pressed") === "true")).toHaveLength(1);
    expect(screen.getByRole("button", { name: "Drop BISHOP" }).getAttribute("aria-pressed")).toBe("true");
    expect(screen.getByRole("button", { name: "Choose BISHOP" }).getAttribute("aria-pressed")).toBe("false");
  });

  it("a_game_without_a_hand_shows_no_hand", () => {
    island(boardView(), { seat: "WHITE" });

    expect(screen.queryByText(/hand/i)).toBeNull();
  });

  it.each([
    ["nothing_is_picked", {}, /select a piece/i],
    ["a_piece_is_selected", { choice: choice({ start: "0,0", chosen: [{ place: "0,0" }] }) }, /choose a highlighted square/i],
    ["a_hand_game_idle", "hand", /or drop one you hold/i],
    [
      "only_answers_are_open",
      { choice: choice({ actions: [{ name: "AGREE", plays: true }, { name: "DISPUTE", plays: true }] }) },
      /^choose an action\.$/i,
    ],
    [
      "a_piece_or_an_action_is_open",
      {
        choice: choice({
          places: [{ name: "0,0", for: "PIECE" }],
          actions: [{ name: "MARK", plays: false }, { name: "DONE", plays: true }],
        }),
      },
      /select a piece, or choose an action/i,
    ],
  ])("hints_what_to_do_when_%s", (_name, overrides, hint) => {
    if (overrides === "hand") island(shogi(), { seat: "SENTE" });
    else island(boardView(overrides as Partial<BoardView>), { seat: "WHITE" });

    expect(screen.getByText(hint)).toBeTruthy();
  });

  it("hints_at_the_chosen_hand_piece", () => {
    island(shogi({ choice: choice({ chosen: [{ held: "BISHOP" }], places: [{ name: "1,1", for: "PLACE" }] }) }));

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

  it("a_skin_may_say_what_the_places_are_for", () => {
    island(
      go({
        choice: choice({
          places: [{ name: "5", for: "PLACE" }],
          actions: [
            { name: "MOVE_1", plays: false },
            { name: "MOVE_3", plays: false },
          ],
        }),
      }),
      { seat: "BLACK", skin: { hint: "Click the point a checker should go to." } },
    );

    expect(screen.getByText("Click the point a checker should go to.")).toBeTruthy();
    expect(screen.queryByText(/on an empty point/i)).toBeNull();
  });

  it("the_skin_hint_gives_way_once_something_is_chosen", () => {
    island(
      go({
        choice: choice({
          chosen: [{ place: "5" }],
          places: [{ name: "5", for: "PLACE" }],
          actions: [
            { name: "MOVE_1", plays: false },
            { name: "MOVE_3", plays: false },
          ],
        }),
      }),
      { seat: "BLACK", skin: { hint: "Click the point a checker should go to." } },
    );

    expect(screen.queryByText("Click the point a checker should go to.")).toBeNull();
    expect(screen.getByText("Choose a highlighted square.")).toBeTruthy();
  });

  it("shows_no_hand_for_a_game_without_one", () => {
    island(go(), { seat: "BLACK" });

    expect(screen.queryByText(/your hand/i)).toBeNull();
  });
  describe("hidden bags", () => {
    const hidden = (owner: string, count?: number): BagView => ({
      name: "HAND",
      owner,
      pieces: {},
      hidden: true,
      ...(count === undefined ? {} : { count }),
    });
    const draft = (bags: BagView[]) =>
      boardView({ players: ["SENTE", "GOTE"], turn: "SENTE", bags, choice: choice({}) });

    it("draws_the_other_hand_as_card_backs_with_its_count_and_no_pieces", () => {
      const { container } = render(() => (
        <PlayIsland
          view={draft([hidden("GOTE", 3), hand("SENTE", { BISHOP: 1 })])}
          skin={null}
          seat="SENTE"
          interactive={false}
          onChoose={vi.fn()}
          onCancel={vi.fn()}
        />
      ));

      const stack = screen.getByRole("img", { name: "GOTE's hand: 3 hidden" });
      expect(stack.querySelectorAll(".island-card-back")).toHaveLength(3);
      expect(screen.getByText("×3")).toBeTruthy();
      // The viewer's own hand stays face up, and nothing of the other is a button.
      expect(screen.getByText(/BISHOP ×1/)).toBeTruthy();
      expect(container.querySelectorAll(".island-card-back")).toHaveLength(3);
      expect(screen.queryByRole("button", { name: /GOTE/ })).toBeNull();
    });

    it("stacks_no_more_than_five_backs_whatever_the_count", () => {
      island(draft([hidden("GOTE", 40)]));

      expect(screen.getByRole("img", { name: "GOTE's hand: 40 hidden" }).querySelectorAll(".island-card-back")).toHaveLength(5);
      expect(screen.getByText("×40")).toBeTruthy();
    });

    it("says_hidden_when_the_game_does_not_show_how_many", () => {
      island(draft([{ name: "DECK", pieces: {}, hidden: true }]));

      expect(screen.getByRole("img", { name: "DECK: hidden" })).toBeTruthy();
      expect(screen.getByText("Hidden")).toBeTruthy();
    });

    it("leaves_out_a_hidden_bag_that_is_known_to_be_empty", () => {
      island(draft([hidden("GOTE", 0), hand("SENTE", {})]));

      expect(screen.queryByRole("img")).toBeNull();
      expect(screen.queryByText(/GOTE/)).toBeNull();
    });

    it("shows_a_hand_face_up_again_once_the_match_is_over_and_the_server_stops_hiding_it", () => {
      island(draft([hand("GOTE", { LANCE: 2 }), hand("SENTE", {})]), { interactive: false });

      expect(screen.queryByRole("img")).toBeNull();
      expect(screen.getByText(/LANCE ×2/)).toBeTruthy();
    });
  });
});
