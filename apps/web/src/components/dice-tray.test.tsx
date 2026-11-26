import { render, screen } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { describe, expect, it } from "vitest";
import type { BoardView } from "../api/types";
import { boardView } from "../test/fixtures";
import { DiceTray, readDice } from "./dice-tray";

const backgammon = (turn: string, dice: Record<string, Record<string, number>>): BoardView =>
  boardView({
    turn,
    players: ["WHITE", "BLACK"],
    bags: [
      { name: "DICE", owner: "BLACK", pieces: dice.BLACK ?? {} },
      { name: "DICE", owner: "WHITE", pieces: dice.WHITE ?? {} },
    ],
  });

const snakes = (turn: string, pips: number): BoardView =>
  boardView({
    turn,
    players: ["RED", "BLUE"],
    bags: [{ name: "TRAY", pieces: pips > 0 ? { PIP: pips } : {} }],
  });

describe("readDice", () => {
  it("reads_nothing_for_a_game_without_dice", () => {
    expect(readDice(boardView())).toBeNull();
    expect(readDice(boardView({ bags: [{ name: "HAND", owner: "WHITE", pieces: {} }] }))).toBeNull();
  });

  it("reads_the_faces_of_the_player_to_moves_dice", () => {
    const reading = readDice(backgammon("WHITE", { WHITE: { D3: 1, D5: 1 }, BLACK: { D6: 1 } }));

    expect(reading?.faces).toEqual([3, 5]);
    expect(reading?.headline).toBe("White’s dice");
    expect(reading?.note).toBe("Each die is one move");
    expect(reading?.seat).toBe("WHITE");
  });

  it("says_a_double_is_four_moves", () => {
    const reading = readDice(backgammon("BLACK", { BLACK: { D4: 4 } }));

    expect(reading?.faces).toEqual([4, 4, 4, 4]);
    expect(reading?.note).toBe("A double: four moves");
  });

  it("asks_for_a_roll_when_the_dice_are_empty", () => {
    const reading = readDice(backgammon("WHITE", {}));

    expect(reading?.faces).toEqual([]);
    expect(reading?.headline).toBe("White to roll");
  });

  it("reads_the_die_last_rolled_from_the_shared_tray_and_credits_the_player_before", () => {
    const reading = readDice(snakes("BLUE", 4));

    expect(reading?.faces).toEqual([4]);
    expect(reading?.headline).toBe("Red rolled 4");
    expect(reading?.note).toBe("Blue to roll");
    expect(reading?.seat).toBe("RED");
  });

  it("credits_the_last_player_when_the_turn_wraps_round", () => {
    expect(readDice(snakes("RED", 2))?.headline).toBe("Blue rolled 2");
  });

  it("asks_for_a_roll_when_the_tray_is_empty", () => {
    expect(readDice(snakes("RED", 0))?.headline).toBe("Red to roll");
  });

  it("changes_its_key_only_when_the_faces_or_the_turn_do", () => {
    const a = readDice(snakes("BLUE", 4))?.key;

    expect(readDice(snakes("BLUE", 4))?.key).toBe(a);
    expect(readDice(snakes("RED", 4))?.key).not.toBe(a);
    expect(readDice(snakes("BLUE", 5))?.key).not.toBe(a);
  });
});

describe("DiceTray", () => {
  it("draws_nothing_for_a_game_without_dice_or_before_there_is_a_board", () => {
    render(() => <DiceTray view={boardView()} />);
    render(() => <DiceTray view={null} />);

    expect(screen.queryByTestId("dice-tray")).toBeNull();
  });

  it("draws_a_die_for_each_face_with_its_pips_and_says_whose_they_are", () => {
    render(() => <DiceTray view={backgammon("WHITE", { WHITE: { D3: 1, D5: 1 } })} />);

    const dice = screen.getAllByRole("img");

    expect(dice.map((die) => die.getAttribute("aria-label"))).toEqual(["A die showing 3", "A die showing 5"]);
    expect(dice[0].querySelectorAll("i.on")).toHaveLength(3);
    expect(dice[1].querySelectorAll("i.on")).toHaveLength(5);
    expect(screen.getByTestId("dice-tray").textContent).toContain("White’s dice");
  });

  it("shows_an_empty_die_to_roll_when_nothing_has_been_rolled", () => {
    render(() => <DiceTray view={snakes("RED", 0)} />);

    expect(screen.queryAllByRole("img")).toHaveLength(0);
    expect(screen.getByTestId("dice-tray").textContent).toContain("Red to roll");
    expect(document.querySelector(".die.ghost")).not.toBeNull();
  });

  it("tumbles_in_a_new_roll_but_not_the_one_it_first_shows", () => {
    const [view, setView] = createSignal(snakes("RED", 0));
    render(() => <DiceTray view={view()} />);
    expect(document.querySelector(".die.tumble")).toBeNull();

    setView(snakes("BLUE", 5));

    expect(document.querySelector(".die.tumble")).not.toBeNull();
    expect(screen.getByTestId("dice-tray").textContent).toContain("Red rolled 5");
  });

  it("does_not_tumble_a_roll_that_was_already_showing_when_the_page_opened", () => {
    render(() => <DiceTray view={snakes("BLUE", 5)} />);

    expect(screen.getAllByRole("img")).toHaveLength(1);
    expect(document.querySelector(".die.tumble")).toBeNull();
  });
});
