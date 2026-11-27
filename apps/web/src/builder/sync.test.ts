import { describe, expect, it } from "vitest";
import { LOCAL, type Remote, type Sync, type SyncEvent, type Syncable, onAccount, reconcile, resume, step, withoutAccountDrafts } from "./sync";

const ALL: Sync[] = [
  LOCAL,
  { kind: "synced", version: 3 },
  { kind: "dirty", version: 3 },
  { kind: "dirty", version: null },
  { kind: "saving", version: 3, again: false },
  { kind: "saving", version: 3, again: true },
  { kind: "saving", version: null, again: false },
  { kind: "failed", version: 3 },
  { kind: "refused", version: 3, reason: "too long" },
  { kind: "conflict", version: 3, current: 5 },
];

const EVENTS: SyncEvent[] = [
  { type: "edit" },
  { type: "upload" },
  { type: "send" },
  { type: "saved", version: 4 },
  { type: "failed" },
  { type: "refused", reason: "too long" },
  { type: "conflict", current: 5 },
  { type: "retry" },
  { type: "keep-mine" },
  { type: "take-theirs", version: 6 },
  { type: "gone" },
];

const name = (s: Sync) => `${s.kind}${"version" in s ? `(${s.version})` : ""}${"again" in s && s.again ? "+again" : ""}`;

/** Every move the machine makes, by state and event. Any pair not listed leaves the state as it is. */
const MOVES: Record<string, Record<string, Sync>> = {
  "synced(3)": { edit: { kind: "dirty", version: 3 }, gone: LOCAL },
  "dirty(3)": { send: { kind: "saving", version: 3, again: false }, gone: LOCAL },
  "dirty(null)": { send: { kind: "saving", version: null, again: false }, gone: LOCAL },
  local: { upload: { kind: "dirty", version: null } },
  "saving(3)": {
    edit: { kind: "saving", version: 3, again: true },
    saved: { kind: "synced", version: 4 },
    failed: { kind: "failed", version: 3 },
    refused: { kind: "refused", version: 3, reason: "too long" },
    conflict: { kind: "conflict", version: 3, current: 5 },
    gone: LOCAL,
  },
  "saving(3)+again": {
    edit: { kind: "saving", version: 3, again: true },
    saved: { kind: "dirty", version: 4 },
    failed: { kind: "failed", version: 3 },
    refused: { kind: "refused", version: 3, reason: "too long" },
    conflict: { kind: "conflict", version: 3, current: 5 },
    gone: LOCAL,
  },
  "saving(null)": {
    edit: { kind: "saving", version: null, again: true },
    saved: { kind: "synced", version: 4 },
    failed: { kind: "failed", version: null },
    refused: { kind: "refused", version: null, reason: "too long" },
    gone: LOCAL,
  },
  "failed(3)": { edit: { kind: "dirty", version: 3 }, retry: { kind: "dirty", version: 3 }, gone: LOCAL },
  "refused(3)": { edit: { kind: "dirty", version: 3 }, gone: LOCAL },
  "conflict(3)": { "keep-mine": { kind: "dirty", version: 5 }, "take-theirs": { kind: "synced", version: 6 }, gone: LOCAL },
};

describe("the sync machine", () => {
  it("answers every state and event as the table says, and leaves it alone otherwise", () => {
    for (const state of ALL) {
      for (const event of EVENTS) {
        const expected = MOVES[name(state)]?.[event.type] ?? state;
        expect(step(state, event), `${name(state)} on ${event.type}`).toEqual(expected);
      }
    }
  });

  it("makes a first save of a new draft without a conflict to have", () => {
    const sending = step(step({ kind: "local" }, { type: "upload" }), { type: "send" });
    expect(step(sending, { type: "conflict", current: 2 })).toEqual(sending);
  });

  it("sends again what was typed while a save was on its way", () => {
    let state = step({ kind: "synced", version: 1 }, { type: "edit" });
    state = step(state, { type: "send" });
    state = step(state, { type: "edit" });
    state = step(state, { type: "saved", version: 2 });
    expect(state).toEqual({ kind: "dirty", version: 2 });
  });

  it("starts a draft read from the browser so that a save that was on its way is sent again", () => {
    expect(resume({ kind: "saving", version: 3, again: false })).toEqual({ kind: "dirty", version: 3 });
    expect(resume({ kind: "failed", version: null })).toEqual({ kind: "dirty", version: null });
    expect(resume({ kind: "refused", version: 3, reason: "x" })).toEqual({ kind: "dirty", version: 3 });
    expect(resume({ kind: "synced", version: 3 })).toEqual({ kind: "synced", version: 3 });
    expect(resume({ kind: "conflict", version: 3, current: 5 })).toEqual({ kind: "conflict", version: 3, current: 5 });
  });

  it("says in words how a draft stands, and nothing for a draft that is only here", async () => {
    const { describe: say } = await import("./sync");
    expect(say(LOCAL)).toBeNull();
    for (const state of ALL.slice(1)) expect(say(state), name(state)).toBeTruthy();
    expect(onAccount(LOCAL)).toBe(false);
    expect(onAccount({ kind: "dirty", version: null })).toBe(true);
  });
});

interface D extends Syncable {
  id: string;
}

const draft = (id: string, over: Partial<D> = {}): D => ({ id, name: id, rules: `r-${id}`, skin: "", sync: LOCAL, shared: null, ...over });
const theirs = (id: string, version: number, over: Partial<Remote> = {}): Remote => ({
  id,
  name: id,
  rules: `r-${id}`,
  skin: "",
  version,
  share_token: null,
  ...over,
});
const make = (r: Remote): D => draft(`new-${r.id}`, { name: r.name, rules: r.rules, skin: r.skin, remote: r.id, sync: { kind: "synced", version: r.version }, shared: r.share_token });

describe("putting the browser's drafts and the account's together", () => {
  it("adds what the account has and the browser has not, as saved", () => {
    const out = reconcile([draft("a")], [theirs("x", 2, { name: "From elsewhere" })], make);
    expect(out.map((d) => d.id)).toEqual(["a", "new-x"]);
    expect(out[1]).toMatchObject({ name: "From elsewhere", remote: "x", sync: { kind: "synced", version: 2 } });
  });

  it("leaves a draft that is only here alone", () => {
    const only = draft("a");
    expect(reconcile([only], [theirs("x", 1)], make)[0]).toBe(only);
  });

  it("takes the account's text when it moved on and the browser's copy was untouched", () => {
    const mine = draft("a", { remote: "r1", sync: { kind: "synced", version: 1 } });
    const [out] = reconcile([mine], [theirs("r1", 4, { rules: "newer", name: "Renamed" })], make);
    expect(out).toMatchObject({ id: "a", rules: "newer", name: "Renamed", sync: { kind: "synced", version: 4 } });
  });

  it("learns the link of a draft that is shared", () => {
    const mine = draft("a", { remote: "r1", sync: { kind: "synced", version: 1 } });
    expect(reconcile([mine], [theirs("r1", 1, { share_token: "tok" })], make)[0].shared).toBe("tok");
  });

  it("makes a conflict of a draft changed on both sides, and keeps the person's text", () => {
    const mine = draft("a", { remote: "r1", rules: "mine", sync: { kind: "dirty", version: 1 } });
    const [out] = reconcile([mine], [theirs("r1", 3, { rules: "theirs" })], make);
    expect(out).toMatchObject({ rules: "mine", sync: { kind: "conflict", version: 1, current: 3 } });
  });

  it("keeps a draft changed here when the account is still where it started", () => {
    const mine = draft("a", { remote: "r1", sync: { kind: "dirty", version: 2 } });
    expect(reconcile([mine], [theirs("r1", 2)], make)[0].sync).toEqual({ kind: "dirty", version: 2 });
  });

  it("drops a copy of a draft that was deleted elsewhere, and keeps work that was never saved as a local draft", () => {
    const copy = draft("a", { remote: "r1", sync: { kind: "synced", version: 1 } });
    const work = draft("b", { remote: "r2", rules: "unsaved", sync: { kind: "dirty", version: 1 }, shared: "tok" });
    const out = reconcile([copy, work], [], make);
    expect(out.map((d) => d.id)).toEqual(["b"]);
    expect(out[0]).toMatchObject({ rules: "unsaved", remote: undefined, sync: LOCAL, shared: null });
  });

  it("brings the other version's number up to date on a conflict still unsettled", () => {
    const mine = draft("a", { remote: "r1", sync: { kind: "conflict", version: 1, current: 2 } });
    expect(reconcile([mine], [theirs("r1", 5)], make)[0].sync).toEqual({ kind: "conflict", version: 1, current: 5 });
  });

  it("leaves only the drafts that were never the account's when the account goes", () => {
    const kept = draft("a");
    expect(withoutAccountDrafts([kept, draft("b", { remote: "r" })])).toEqual([kept]);
  });
});
