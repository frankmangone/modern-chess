import { beforeEach, describe, expect, it } from "vitest";
import { fileName, loadDrafts, newDraft, saveDrafts, uniqueName } from "./drafts";

beforeEach(() => localStorage.clear());

describe("drafts", () => {
  it("starts empty when the browser kept nothing", () => {
    expect(loadDrafts()).toEqual({ drafts: [], current: null });
  });

  it("keeps what was saved, and which one was on show", () => {
    const a = newDraft("A", "{}");
    const b = newDraft("B", "{ }", '{"pieces":{}}');

    expect(saveDrafts({ drafts: [a, b], current: b.id })).toBe(true);

    expect(loadDrafts()).toEqual({ drafts: [a, b], current: b.id });
  });

  it("falls back to the first draft when the one on show is gone", () => {
    const a = newDraft("A", "{}");
    saveDrafts({ drafts: [a], current: "nope" });

    expect(loadDrafts().current).toBe(a.id);
  });

  it("leaves out what is not a draft, and survives text that is not JSON", () => {
    localStorage.setItem("hubbable.drafts", JSON.stringify({ drafts: [{ id: 1 }, newDraft("A", "{}")], current: null }));
    expect(loadDrafts().drafts.map((d) => d.name)).toEqual(["A"]);

    localStorage.setItem("hubbable.drafts", "{not json");
    expect(loadDrafts()).toEqual({ drafts: [], current: null });
  });

  it("gives drafts different ids", () => {
    expect(newDraft("A", "").id).not.toBe(newDraft("A", "").id);
  });

  it("makes a name different from the others by numbering it", () => {
    expect(uniqueName("Chess", [])).toBe("Chess");
    expect(uniqueName("Chess", ["Chess"])).toBe("Chess 2");
    expect(uniqueName("Chess", ["Chess", "Chess 2"])).toBe("Chess 3");
  });

  it("names a downloaded file after the game", () => {
    expect(fileName("Pong hau k'i: Fan")).toBe("pong-hau-k-i-fan.json");
    expect(fileName("???")).toBe("game.json");
  });
});
