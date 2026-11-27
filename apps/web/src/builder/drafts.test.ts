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

  it("reads a draft kept before accounts existed as one that is only here", () => {
    localStorage.setItem(
      "hubbable.drafts",
      JSON.stringify({ drafts: [{ id: "d", name: "Old", rules: "{}", skin: "", updated: 1 }], current: "d" }),
    );

    expect(loadDrafts().drafts[0]).toMatchObject({ sync: { kind: "local" }, shared: null });
    expect(loadDrafts().drafts[0].remote).toBeUndefined();
  });

  it("keeps how a draft stands with the account, and sends again a save that was on its way", () => {
    const base = newDraft("A", "{}");
    const kept = [
      { ...base, id: "1", remote: "r1", sync: { kind: "synced", version: 2 }, shared: "tok" },
      { ...base, id: "2", remote: "r2", sync: { kind: "saving", version: 4, again: true } },
      { ...base, id: "3", sync: { kind: "dirty", version: null } },
      { ...base, id: "4", remote: "r4", sync: { kind: "conflict", version: 1, current: 3 } },
    ];
    saveDrafts({ drafts: kept as never, current: "1" });

    const [synced, saving, waiting, conflict] = loadDrafts().drafts;

    expect(synced).toMatchObject({ remote: "r1", sync: { kind: "synced", version: 2 }, shared: "tok" });
    expect(saving.sync).toEqual({ kind: "dirty", version: 4 });
    expect(waiting.sync).toEqual({ kind: "dirty", version: null });
    expect(conflict.sync).toEqual({ kind: "conflict", version: 1, current: 3 });
  });

  it("does not believe a draft is on the account when it has no id there, or has a state it does not know", () => {
    const base = newDraft("A", "{}");
    const kept = [
      { ...base, id: "1", sync: { kind: "synced", version: 2 } },
      { ...base, id: "2", remote: "r", sync: { kind: "made-up" } },
      { ...base, id: "3", remote: "r", sync: { kind: "synced", version: "two" } },
      { ...base, id: "4", remote: "r", sync: { kind: "conflict", version: 1 } },
    ];
    saveDrafts({ drafts: kept as never, current: "1" });

    expect(loadDrafts().drafts.map((d) => d.sync.kind)).toEqual(["local", "local", "local", "local"]);
  });
});
