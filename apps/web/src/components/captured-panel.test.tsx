import { render, screen } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { boardView } from "../test/fixtures";
import { CapturedPanel } from "./captured-panel";

const chess = (overrides = {}) =>
  boardView({
    players: ["WHITE", "BLACK"],
    captured: { WHITE: {}, BLACK: {} },
    ...overrides,
  });

function panel(view = chess()) {
  render(() => <CapturedPanel view={view} skin={null} />);
}

describe("CapturedPanel", () => {
  it("says_so_while_nobody_has_lost_a_piece", () => {
    panel();

    expect(screen.getAllByText(/nothing lost yet/i)).toHaveLength(2);
    expect(screen.getByLabelText("Pieces lost: 0 in total")).toBeTruthy();
  });

  it("lists_what_each_side_has_lost_with_a_total", () => {
    panel(chess({ captured: { WHITE: { PAWN: 1 }, BLACK: { PAWN: 2, ROOK: 1 } } }));

    expect(screen.getByLabelText("BLACK lost: PAWN ×2, ROOK ×1")).toBeTruthy();
    expect(screen.getByLabelText("WHITE lost: PAWN ×1")).toBeTruthy();
    expect(screen.getByLabelText("Pieces lost: 4 in total")).toBeTruthy();
  });

  it("shows_which_side_leads_in_material", () => {
    panel(chess({ material: { WHITE: 39, BLACK: 36 } }));

    expect(screen.getByText("WHITE +3")).toBeTruthy();
  });

  it("shows_an_even_count_when_the_material_is_level", () => {
    panel(chess({ material: { WHITE: 39, BLACK: 39 } }));

    expect(screen.getByText("= 0")).toBeTruthy();
  });

  it("leaves_the_material_out_when_the_game_gives_its_pieces_no_value", () => {
    panel();

    expect(screen.queryByText(/material/i)).toBeNull();
  });
});
