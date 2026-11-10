import { render } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { PieceFace } from "./piece-face";

const players = ["RED", "BLACK"];

describe("PieceFace", () => {
  it("draws_a_round_tile_with_the_player_s_colour_for_a_disc_skin", () => {
    const skin = {
      pieceStyle: "disc",
      colours: { RED: "#c0392b" },
      pieces: { SOLDIER: { RED: "兵", BLACK: "卒" } },
    };

    const { container } = render(() => <PieceFace skin={skin} code="SOLDIER" player="RED" players={players} />);

    const tile = container.querySelector<HTMLElement>(".island-piece");
    expect(tile?.classList.contains("island-disc")).toBe(true);
    expect(tile?.textContent).toBe("兵");
    expect(tile?.style.getPropertyValue("--mcb-ring")).toBe("#c0392b");
  });

  it("keeps_the_square_tile_when_the_skin_does_not_ask_for_discs", () => {
    const { container } = render(() => (
      <PieceFace skin={{ pieces: { ROOK: "♜" } }} code="ROOK" player="RED" players={players} />
    ));

    expect(container.querySelector(".island-disc")).toBeNull();
  });

  it("draws_a_stone_from_an_svg_face_without_a_disc_ring", () => {
    const skin = {
      pieces: { STONE: { RED: { svg: '<svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="4"/></svg>' } } },
    };

    const { container } = render(() => <PieceFace skin={skin} code="STONE" player="RED" players={players} />);

    expect(container.querySelector(".island-piece svg circle")).not.toBeNull();
  });
});
