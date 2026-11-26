import { describe, expect, it } from "vitest";
import { showsBoard } from "./board-panel";

describe("showsBoard", () => {
  it("is on unless the skin turns it off", () => {
    expect(showsBoard(null)).toBe(true);
    expect(showsBoard({})).toBe(true);
    expect(showsBoard({ board_panel: true })).toBe(true);
    expect(showsBoard({ board_panel: false })).toBe(false);
  });

  it("ignores a value that is not false", () => {
    expect(showsBoard({ board_panel: 0 })).toBe(true);
    expect(showsBoard({ board_panel: "no" })).toBe(true);
  });
});
