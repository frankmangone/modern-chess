import { render, screen } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { RecordTag, recordSentence } from "./record-tag";

describe("recordSentence", () => {
  it("says_each_count_and_uses_the_singular_for_one", () => {
    expect(recordSentence({ won: 12, drawn: 3, lost: 5 })).toBe("12 wins, 3 draws, 5 losses");
    expect(recordSentence({ won: 1, drawn: 1, lost: 1 })).toBe("1 win, 1 draw, 1 loss");
    expect(recordSentence({ won: 0, drawn: 0, lost: 0 })).toBe("0 wins, 0 draws, 0 losses");
  });
});

describe("RecordTag", () => {
  it("is_short_to_look_at_and_a_sentence_to_read_aloud", () => {
    render(() => <RecordTag record={{ won: 12, drawn: 3, lost: 5 }} />);

    expect(screen.getByText("12W 3D 5L")).toBeTruthy();
    expect(screen.getByText("12 wins, 3 draws, 5 losses")).toBeTruthy();
  });

  it("says_new_for_someone_with_no_finished_match", () => {
    render(() => <RecordTag record={{ won: 0, drawn: 0, lost: 0 }} />);

    expect(screen.getByText("New")).toBeTruthy();
    expect(screen.getByText("No finished matches on this game yet")).toBeTruthy();
    expect(screen.queryByText(/0W/)).toBeNull();
  });
});
