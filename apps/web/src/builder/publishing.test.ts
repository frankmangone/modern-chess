import { describe, expect, it } from "vitest";
import {
  CLOSED,
  type Publishing,
  type PublishingEvent,
  describeDone,
  formOpen,
  listingProblem,
  step,
  whyNotYet,
} from "./publishing";
import { LOCAL, type Sync } from "./sync";

const STATES: Publishing[] = [
  CLOSED,
  { kind: "open" },
  { kind: "sending" },
  { kind: "refused", reason: "no" },
  { kind: "done", filed: { status: "in_review" }, name: "X" },
];
const EVENTS: PublishingEvent[] = [
  { type: "open" },
  { type: "close" },
  { type: "send" },
  { type: "filed", filed: { status: "published", slug: "x" }, name: "X" },
  { type: "refused", reason: "why" },
  { type: "dismiss" },
];

/** The whole table, written out: the one place every state and event is read at once. */
function expected(state: Publishing, event: PublishingEvent): Publishing {
  const kind = state.kind;
  switch (event.type) {
    case "open":
      return kind === "closed" ? { kind: "open" } : state;
    case "close":
      return kind === "open" || kind === "refused" ? CLOSED : state;
    case "send":
      return kind === "open" || kind === "refused" ? { kind: "sending" } : state;
    case "filed":
      return kind === "sending" ? { kind: "done", filed: event.filed, name: event.name } : state;
    case "refused":
      return kind === "sending" ? { kind: "refused", reason: event.reason } : state;
    case "dismiss":
      return kind === "done" ? CLOSED : state;
  }
}

describe("publishing", () => {
  it("answers every state and event as the table says", () => {
    for (const state of STATES) for (const event of EVENTS) expect(step(state, event), `${state.kind} on ${event.type}`).toEqual(expected(state, event));
  });

  it("goes from open to sending to done, and a refusal brings the form back with its reason", () => {
    let now = step(CLOSED, { type: "open" });
    now = step(now, { type: "send" });
    now = step(now, { type: "refused", reason: "The tagline is too long." });
    expect(now).toEqual({ kind: "refused", reason: "The tagline is too long." });
    now = step(step(now, { type: "send" }), { type: "filed", filed: { status: "in_review" }, name: "March" });
    expect(now.kind).toBe("done");
    expect(step(now, { type: "dismiss" })).toEqual(CLOSED);
  });

  it("shows the form while it is open, being sent, or refused, and never when closed or done", () => {
    expect(STATES.map(formOpen)).toEqual([false, true, true, true, false]);
  });

  it("says what a draft needs before it can be handed in", () => {
    const synced: Sync = { kind: "synced", version: 3 };
    expect(whyNotYet({ sync: LOCAL }, true, true)).toBe("Save it to your account first.");
    expect(whyNotYet({ remote: "r", sync: synced }, false, true)).toBe("The engine has to read the rules first.");
    expect(whyNotYet({ remote: "r", sync: synced }, true, false)).toBe("The look has to be readable first.");
    expect(whyNotYet({ remote: "r", sync: { kind: "dirty", version: 3 } }, true, true)).toBe("Waiting for your latest changes to be saved.");
    expect(whyNotYet({ remote: "r", sync: { kind: "conflict", version: 3, current: 4 } }, true, true)).toBe("Choose which version to keep first.");
    expect(whyNotYet({ remote: "r", sync: synced }, true, true)).toBeNull();
  });

  it("holds a listing to the words the server asks for", () => {
    const ok = { tagline: "Two in a row", description: "A game for two.", licence: "CC-BY-SA-4.0" };
    expect(listingProblem(ok, true)).toBeNull();
    expect(listingProblem({ ...ok, tagline: "  " }, true)).toMatch(/in a line/);
    expect(listingProblem({ ...ok, tagline: "x".repeat(141) }, true)).toMatch(/too long/);
    expect(listingProblem({ ...ok, description: "" }, true)).toMatch(/Describe/);
    expect(listingProblem({ ...ok, description: "x".repeat(2001) }, true)).toMatch(/too long/);
    expect(listingProblem(ok, false)).toMatch(/CC BY-SA 4.0/);
  });

  it("tells a person what happened in words", () => {
    expect(describeDone({ kind: "done", filed: { status: "published", slug: "x" }, name: "March" })).toBe("“March” is on the hub.");
    expect(describeDone({ kind: "done", filed: { status: "in_review" }, name: "March" })).toMatch(/looked at/);
  });
});
