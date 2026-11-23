import { cleanup, render, screen } from "@solidjs/testing-library";
import { afterEach, describe, expect, it } from "vitest";
import { HeroBoard, heroCells } from "./hero-board";

afterEach(cleanup);

describe("HeroBoard", () => {
  it("draws_a_seven_by_seven_board_with_its_corners_and_middle_cut_away", () => {
    const cells = heroCells();

    expect(cells).toHaveLength(49);
    const holes = cells.flatMap((cell, index) => (cell.hole ? [index] : []));
    expect(holes).toEqual([0, 6, 24, 42, 48]);
  });

  it("marks_the_top_row_as_a_zone_except_where_it_is_cut_away", () => {
    const zone = heroCells().flatMap((cell, index) => (cell.zone ? [index] : []));

    expect(zone).toEqual([1, 2, 3, 4, 5]);
  });

  it("alternates_light_and_dark_squares_like_a_chequerboard", () => {
    const cells = heroCells();

    expect(cells[0]?.dark).toBe(false);
    expect(cells[1]?.dark).toBe(true);
    expect(cells[7]?.dark).toBe(true);
  });

  it("is_hidden_from_assistive_technology_and_holds_every_square", () => {
    render(() => <HeroBoard />);
    const board = screen.getByTestId("hero-board");

    expect(board.getAttribute("aria-hidden")).toBe("true");
    expect(board.querySelectorAll(".hero-cell")).toHaveLength(49);
    expect(board.querySelectorAll(".hero-cell.hole")).toHaveLength(5);
    expect(board.querySelectorAll(".hero-cell.zone")).toHaveLength(5);
  });
});
